use crate::db::{get_cached_string, invalidate_cache, set_cached_string, AppState};
use crate::models::{
    random_colors, CreateNotePayload, DataResponse, DeleteResponse, ErrorResponse, MutationResponse,
    Note, NoteSummary, ReplaceNotePayload, UpdateNotePayload,
};
use actix_web::{web, HttpResponse, Responder};
use log::error;
use uuid::Uuid;
use validator::Validate;

const CACHE_KEY_LIST: &str = "cache:notes:list";
const CACHE_TTL_SECS: u64 = 60;

fn format_validation_errors(err: validator::ValidationErrors) -> String {
    let mut messages: Vec<String> = Vec::new();
    for (_field, errors) in err.field_errors() {
        for e in errors {
            if let Some(msg) = &e.message {
                messages.push(msg.to_string());
            } else {
                messages.push(format!("Invalid field value: {}", e.code));
            }
        }
    }
    if messages.is_empty() {
        "Invalid input data".to_string()
    } else {
        messages.join(", ")
    }
}

pub async fn health_check(data: web::Data<AppState>) -> impl Responder {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&data.db)
        .await
        .is_ok();

    let redis_ok = if let Some(ref pool) = data.redis {
        match pool.get().await {
            Ok(mut conn) => redis::cmd("PING").query_async::<String>(&mut conn).await.is_ok(),
            Err(_) => false,
        }
    } else {
        true
    };

    if db_ok && redis_ok {
        HttpResponse::Ok().json(serde_json::json!({
            "status": "healthy",
            "database": "connected",
            "redis": if data.redis.is_some() { "connected" } else { "disabled" }
        }))
    } else {
        HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "status": "unhealthy",
            "database": if db_ok { "connected" } else { "disconnected" },
            "redis": if redis_ok { "connected" } else { "disconnected" }
        }))
    }
}

pub async fn not_found_handler() -> impl Responder {
    HttpResponse::NotFound().json(ErrorResponse::new(404, "Requested resource not found"))
}

pub async fn list_notes(data: web::Data<AppState>) -> impl Responder {
    if let Some(cached) = get_cached_string(&data.redis, CACHE_KEY_LIST).await {
        if let Ok(cached_notes) = serde_json::from_str::<Vec<NoteSummary>>(&cached) {
            return HttpResponse::Ok().json(DataResponse { data: cached_notes });
        }
    }

    let result = sqlx::query_as::<_, Note>(
        "SELECT id, title, content, color_front, color_back, created_at, updated_at FROM notes ORDER BY created_at DESC"
    )
    .fetch_all(&data.db)
    .await;

    match result {
        Ok(notes) => {
            let summaries: Vec<NoteSummary> = notes.into_iter().map(|n| n.to_summary(120)).collect();
            if let Ok(json_str) = serde_json::to_string(&summaries) {
                set_cached_string(&data.redis, CACHE_KEY_LIST, &json_str, CACHE_TTL_SECS).await;
            }
            HttpResponse::Ok().json(DataResponse { data: summaries })
        }
        Err(err) => {
            error!("Failed to fetch notes: {}", err);
            HttpResponse::InternalServerError().json(ErrorResponse::new(500, "Failed to retrieve notes"))
        }
    }
}

pub async fn get_note_by_id(data: web::Data<AppState>, path: web::Path<Uuid>) -> impl Responder {
    let note_id = path.into_inner();
    let cache_key = format!("cache:note:{}", note_id);

    if let Some(cached) = get_cached_string(&data.redis, &cache_key).await {
        if let Ok(cached_note) = serde_json::from_str::<Note>(&cached) {
            return HttpResponse::Ok().json(DataResponse { data: cached_note });
        }
    }

    let result = sqlx::query_as::<_, Note>(
        "SELECT id, title, content, color_front, color_back, created_at, updated_at FROM notes WHERE id = $1"
    )
    .bind(note_id)
    .fetch_optional(&data.db)
    .await;

    match result {
        Ok(Some(note)) => {
            if let Ok(json_str) = serde_json::to_string(&note) {
                set_cached_string(&data.redis, &cache_key, &json_str, CACHE_TTL_SECS).await;
            }
            HttpResponse::Ok().json(DataResponse { data: note })
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse::new(404, format!("Note with id {} not found", note_id))),
        Err(err) => {
            error!("Failed to fetch note {}: {}", note_id, err);
            HttpResponse::InternalServerError().json(ErrorResponse::new(500, "Failed to retrieve note"))
        }
    }
}

pub async fn create_note(
    data: web::Data<AppState>,
    mut payload: web::Json<CreateNotePayload>,
) -> impl Responder {
    payload.trim_and_clean();

    if let Err(err) = payload.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(400, format_validation_errors(err)));
    }

    let (def_front, def_back) = random_colors();
    let color_front = payload.color_front.clone().unwrap_or_else(|| def_front.to_string());
    let color_back = payload.color_back.clone().unwrap_or_else(|| def_back.to_string());

    let result = sqlx::query_as::<_, Note>(
        r#"
        INSERT INTO notes (title, content, color_front, color_back)
        VALUES ($1, $2, $3, $4)
        RETURNING id, title, content, color_front, color_back, created_at, updated_at
        "#
    )
    .bind(&payload.title)
    .bind(&payload.content)
    .bind(&color_front)
    .bind(&color_back)
    .fetch_one(&data.db)
    .await;

    match result {
        Ok(note) => {
            invalidate_cache(&data.redis, &[CACHE_KEY_LIST]).await;
            HttpResponse::Created().json(MutationResponse {
                status: 201,
                success: true,
                message: "Note created successfully".to_string(),
                note_id: note.id.to_string(),
                data: note,
            })
        }
        Err(err) => {
            error!("Failed to create note: {}", err);
            HttpResponse::InternalServerError().json(ErrorResponse::new(500, "Failed to create note"))
        }
    }
}

pub async fn replace_note(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    mut payload: web::Json<ReplaceNotePayload>,
) -> impl Responder {
    payload.trim_and_clean();

    if let Err(err) = payload.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(400, format_validation_errors(err)));
    }

    let (def_front, def_back) = random_colors();
    let color_front = payload.color_front.clone().unwrap_or_else(|| def_front.to_string());
    let color_back = payload.color_back.clone().unwrap_or_else(|| def_back.to_string());

    let note_id = path.into_inner();
    let result = sqlx::query_as::<_, Note>(
        r#"
        UPDATE notes
        SET title = $1, content = $2, color_front = $3, color_back = $4, updated_at = CURRENT_TIMESTAMP
        WHERE id = $5
        RETURNING id, title, content, color_front, color_back, created_at, updated_at
        "#
    )
    .bind(&payload.title)
    .bind(&payload.content)
    .bind(&color_front)
    .bind(&color_back)
    .bind(note_id)
    .fetch_optional(&data.db)
    .await;

    match result {
        Ok(Some(updated_note)) => {
            let note_cache_key = format!("cache:note:{}", note_id);
            invalidate_cache(&data.redis, &[CACHE_KEY_LIST, &note_cache_key]).await;
            HttpResponse::Ok().json(MutationResponse {
                status: 200,
                success: true,
                message: "Note updated successfully".to_string(),
                note_id: updated_note.id.to_string(),
                data: updated_note,
            })
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse::new(404, format!("Note with id {} not found", note_id))),
        Err(err) => {
            error!("Failed to update note {}: {}", note_id, err);
            HttpResponse::InternalServerError().json(ErrorResponse::new(500, "Failed to update note"))
        }
    }
}

pub async fn update_note(
    data: web::Data<AppState>,
    path: web::Path<Uuid>,
    mut payload: web::Json<UpdateNotePayload>,
) -> impl Responder {
    payload.trim_and_clean();

    if let Err(err) = payload.validate() {
        return HttpResponse::BadRequest().json(ErrorResponse::new(400, format_validation_errors(err)));
    }

    let note_id = path.into_inner();

    let existing = match sqlx::query_as::<_, Note>(
        "SELECT id, title, content, color_front, color_back, created_at, updated_at FROM notes WHERE id = $1"
    )
    .bind(note_id)
    .fetch_optional(&data.db)
    .await
    {
        Ok(Some(note)) => note,
        Ok(None) => return HttpResponse::NotFound().json(ErrorResponse::new(404, format!("Note with id {} not found", note_id))),
        Err(err) => {
            error!("Failed to query note {}: {}", note_id, err);
            return HttpResponse::InternalServerError().json(ErrorResponse::new(500, "Database query failed"));
        }
    };

    let title = payload.title.as_ref().unwrap_or(&existing.title);
    let content = payload.content.as_ref().unwrap_or(&existing.content);
    let color_front = payload.color_front.as_ref().unwrap_or(&existing.color_front);
    let color_back = payload.color_back.as_ref().unwrap_or(&existing.color_back);

    let result = sqlx::query_as::<_, Note>(
        r#"
        UPDATE notes
        SET title = $1, content = $2, color_front = $3, color_back = $4, updated_at = CURRENT_TIMESTAMP
        WHERE id = $5
        RETURNING id, title, content, color_front, color_back, created_at, updated_at
        "#
    )
    .bind(title)
    .bind(content)
    .bind(color_front)
    .bind(color_back)
    .bind(note_id)
    .fetch_optional(&data.db)
    .await;

    match result {
        Ok(Some(updated_note)) => {
            let note_cache_key = format!("cache:note:{}", note_id);
            invalidate_cache(&data.redis, &[CACHE_KEY_LIST, &note_cache_key]).await;
            HttpResponse::Ok().json(MutationResponse {
                status: 200,
                success: true,
                message: "Note updated successfully".to_string(),
                note_id: updated_note.id.to_string(),
                data: updated_note,
            })
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse::new(404, format!("Note with id {} not found", note_id))),
        Err(err) => {
            error!("Failed to patch note {}: {}", note_id, err);
            HttpResponse::InternalServerError().json(ErrorResponse::new(500, "Failed to update note"))
        }
    }
}

pub async fn delete_note(data: web::Data<AppState>, path: web::Path<Uuid>) -> impl Responder {
    let note_id = path.into_inner();

    let result = sqlx::query("DELETE FROM notes WHERE id = $1")
        .bind(note_id)
        .execute(&data.db)
        .await;

    match result {
        Ok(query_result) if query_result.rows_affected() > 0 => {
            let note_cache_key = format!("cache:note:{}", note_id);
            invalidate_cache(&data.redis, &[CACHE_KEY_LIST, &note_cache_key]).await;
            HttpResponse::Ok().json(DeleteResponse {
                status: 200,
                success: true,
                message: "Note deleted successfully".to_string(),
            })
        }
        Ok(_) => HttpResponse::NotFound().json(ErrorResponse::new(404, format!("Note with id {} not found", note_id))),
        Err(err) => {
            error!("Failed to delete note {}: {}", note_id, err);
            HttpResponse::InternalServerError().json(ErrorResponse::new(500, "Failed to delete note"))
        }
    }
}

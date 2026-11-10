mod db;
mod handlers;
mod models;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenvy::dotenv;
use log::info;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16 number");

    info!("Connecting to PostgreSQL database...");
    let db_pool = db::init_db_pool().await;

    info!("Running database migrations...");
    db::run_migrations(&db_pool).await;

    info!("Initializing Redis connection pool...");
    let redis_pool = db::init_redis_pool();

    let app_state = db::AppState {
        db: db_pool,
        redis: redis_pool,
    };

    info!("Starting HTTP server on {}:{}", host, port);

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(app_state.clone()))
            .route("/api/healty", web::get().to(handlers::health_check))
            .service(
                web::scope("/api/v1")
                    .route("/lists", web::get().to(handlers::list_notes))
                    .route("/note", web::post().to(handlers::create_note))
                    .route("/note/{id}", web::get().to(handlers::get_note_by_id))
                    .route("/note/{id}", web::put().to(handlers::replace_note))
                    .route("/note/{id}", web::patch().to(handlers::update_note))
                    .route("/note/{id}", web::delete().to(handlers::delete_note)),
            )
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

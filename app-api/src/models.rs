use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub color_front: String,
    pub color_back: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteSummary {
    pub id: Uuid,
    pub title: String,
    pub content_preview: String,
    pub color_front: String,
    pub color_back: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Note {
    pub fn to_summary(&self, max_length: usize) -> NoteSummary {
        let content_preview = if self.content.chars().count() > max_length {
            let truncated: String = self.content.chars().take(max_length).collect();
            format!("{}...", truncated)
        } else {
            self.content.clone()
        };

        NoteSummary {
            id: self.id,
            title: self.title.clone(),
            content_preview,
            color_front: self.color_front.clone(),
            color_back: self.color_back.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateNotePayload {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: String,

    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: String,

    #[validate(length(min = 1, max = 50, message = "color_front cannot be longer than 50 characters"))]
    pub color_front: Option<String>,

    #[validate(length(min = 1, max = 50, message = "color_back cannot be longer than 50 characters"))]
    pub color_back: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReplaceNotePayload {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: String,

    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: String,

    #[validate(length(min = 1, max = 50, message = "color_front cannot be longer than 50 characters"))]
    pub color_front: String,

    #[validate(length(min = 1, max = 50, message = "color_back cannot be longer than 50 characters"))]
    pub color_back: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateNotePayload {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: Option<String>,

    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: Option<String>,

    #[validate(length(min = 1, max = 50, message = "color_front cannot be longer than 50 characters"))]
    pub color_front: Option<String>,

    #[validate(length(min = 1, max = 50, message = "color_back cannot be longer than 50 characters"))]
    pub color_back: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn ok_msg(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

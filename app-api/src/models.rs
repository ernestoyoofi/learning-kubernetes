use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

const COLOR_PALETTES: &[(&str, &str)] = &[
    ("#1f2937", "#fef08a"), // dark gray / yellow
    ("#1e293b", "#bfdbfe"), // slate / light blue
    ("#064e3b", "#bbf7d0"), // dark green / mint
    ("#701a75", "#fbcfe8"), // dark purple / pink
    ("#7c2d12", "#fed7aa"), // dark orange / peach
    ("#1e1b4b", "#e0e7ff"), // indigo / soft violet
    ("#312e81", "#c7d2fe"), // blue-indigo / soft blue
    ("#134e4a", "#99f6e4"), // teal / soft teal
    ("#18181b", "#f4f4f5"), // zinc dark / off-white
    ("#7f1d1d", "#fecaca"), // dark red / soft rose
];

pub fn random_colors() -> (&'static str, &'static str) {
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..COLOR_PALETTES.len());
    COLOR_PALETTES[idx]
}

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
    #[validate(length(min = 1, max = 80, message = "Title must be between 1 and 80 characters"))]
    pub title: String,

    #[validate(length(min = 1, max = 512, message = "Content must be between 1 and 512 characters"))]
    pub content: String,

    #[validate(length(min = 1, max = 50, message = "color_front cannot exceed 50 characters"))]
    pub color_front: Option<String>,

    #[validate(length(min = 1, max = 50, message = "color_back cannot exceed 50 characters"))]
    pub color_back: Option<String>,
}

impl CreateNotePayload {
    pub fn trim_and_clean(&mut self) {
        self.title = self.title.trim().to_string();
        self.content = self.content.trim().to_string();
        self.color_front = self.color_front.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.color_back = self.color_back.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReplaceNotePayload {
    #[validate(length(min = 1, max = 80, message = "Title must be between 1 and 80 characters"))]
    pub title: String,

    #[validate(length(min = 1, max = 512, message = "Content must be between 1 and 512 characters"))]
    pub content: String,

    #[validate(length(min = 1, max = 50, message = "color_front cannot exceed 50 characters"))]
    pub color_front: Option<String>,

    #[validate(length(min = 1, max = 50, message = "color_back cannot exceed 50 characters"))]
    pub color_back: Option<String>,
}

impl ReplaceNotePayload {
    pub fn trim_and_clean(&mut self) {
        self.title = self.title.trim().to_string();
        self.content = self.content.trim().to_string();
        self.color_front = self.color_front.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.color_back = self.color_back.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateNotePayload {
    #[validate(length(min = 1, max = 80, message = "Title must be between 1 and 80 characters"))]
    pub title: Option<String>,

    #[validate(length(min = 1, max = 512, message = "Content must be between 1 and 512 characters"))]
    pub content: Option<String>,

    #[validate(length(min = 1, max = 50, message = "color_front cannot exceed 50 characters"))]
    pub color_front: Option<String>,

    #[validate(length(min = 1, max = 50, message = "color_back cannot exceed 50 characters"))]
    pub color_back: Option<String>,
}

impl UpdateNotePayload {
    pub fn trim_and_clean(&mut self) {
        self.title = self.title.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.content = self.content.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.color_front = self.color_front.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.color_back = self.color_back.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    }
}

#[derive(Debug, Serialize)]
pub struct DataResponse<T> {
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct MutationResponse<T> {
    pub status: u16,
    pub success: bool,
    pub message: String,
    pub note_id: String,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub status: u16,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: u16,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

pub const TITLE_MAX_LEN: usize = 255;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTodo {
    pub title: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoPatch {
    pub title: Option<String>,
    pub completed: Option<bool>,
}

/// タイトルの業務ルール: 前後空白を除去し、1〜255 文字。
pub fn normalize_title(raw: &str) -> Result<String, AppError> {
    let title = raw.trim();
    if title.is_empty() {
        return Err(AppError::Validation("title must not be empty".into()));
    }
    if title.chars().count() > TITLE_MAX_LEN {
        return Err(AppError::Validation(format!(
            "title must be at most {TITLE_MAX_LEN} characters"
        )));
    }
    Ok(title.to_string())
}

impl NewTodo {
    pub fn validate(self) -> Result<Self, AppError> {
        Ok(Self {
            title: normalize_title(&self.title)?,
        })
    }
}

impl TodoPatch {
    pub fn validate(self) -> Result<Self, AppError> {
        if self.title.is_none() && self.completed.is_none() {
            return Err(AppError::Validation(
                "at least one field is required".into(),
            ));
        }
        let title = self.title.as_deref().map(normalize_title).transpose()?;
        Ok(Self {
            title,
            completed: self.completed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_trimmed() {
        assert_eq!(normalize_title("  hello ").unwrap(), "hello");
    }

    #[test]
    fn empty_title_is_rejected() {
        assert!(matches!(
            normalize_title("   "),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn title_length_boundary() {
        assert!(normalize_title(&"あ".repeat(TITLE_MAX_LEN)).is_ok());
        assert!(normalize_title(&"あ".repeat(TITLE_MAX_LEN + 1)).is_err());
    }

    #[test]
    fn empty_patch_is_rejected() {
        assert!(TodoPatch::default().validate().is_err());
    }
}

use async_graphql::{InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, FromRow)]
#[graphql(rename_fields = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, InputObject, Validate)]
#[graphql(rename_fields = "camelCase")]
pub struct CreateUserInput {
    #[validate(email(message = "Invalid email format"))]
    #[validate(length(min = 1, message = "Email is required"))]
    pub email: String,
    
    #[validate(length(min = 1, max = 100, message = "Name must be between 1 and 100 characters"))]
    pub name: String,
}

#[derive(Debug, InputObject, Validate)]
#[graphql(rename_fields = "camelCase")]
pub struct UpdateUserInput {
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    
    #[validate(length(min = 1, max = 100, message = "Name must be between 1 and 100 characters"))]
    pub name: Option<String>,
    
    pub is_active: Option<bool>,
}

#[derive(Debug, InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct UserFilter {
    pub email: Option<String>,
    pub name: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, InputObject)]
pub struct PaginationInput {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

impl Default for PaginationInput {
    fn default() -> Self {
        Self {
            offset: Some(0),
            limit: Some(10),
        }
    }
}
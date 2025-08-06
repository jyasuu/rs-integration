use crate::{
    database::Database,
    errors::{AppError, AppResult},
    models::user::{CreateUserInput, UpdateUserInput, User, UserFilter, PaginationInput},
};
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

#[derive(Clone)]
pub struct UserService {
    db: Database,
}

impl UserService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn create_user(&self, input: CreateUserInput) -> AppResult<User> {
        input.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        let id = Uuid::new_v4();
        let now = Utc::now();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, name, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, name, is_active, created_at, updated_at
            "#
        )
        .bind(id)
        .bind(input.email)
        .bind(input.name)
        .bind(true)
        .bind(now)
        .bind(now)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, name, is_active, created_at, updated_at
            FROM users
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.db.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User with id {} not found", id)))?;

        Ok(user)
    }

    pub async fn get_users(&self, filter: Option<UserFilter>, pagination: PaginationInput) -> AppResult<Vec<User>> {
        let limit = pagination.limit.unwrap_or(10);
        let offset = pagination.offset.unwrap_or(0);

        let users = match filter {
            Some(f) => {
                let email_filter = f.email.unwrap_or_else(|| "%".to_string());
                let name_filter = f.name.unwrap_or_else(|| "%".to_string());
                
                if let Some(is_active) = f.is_active {
                    sqlx::query_as::<_, User>(
                        r#"
                        SELECT id, email, name, is_active, created_at, updated_at
                        FROM users
                        WHERE email ILIKE $1 AND name ILIKE $2 AND is_active = $3
                        ORDER BY created_at DESC
                        LIMIT $4 OFFSET $5
                        "#
                    )
                    .bind(format!("%{}%", email_filter))
                    .bind(format!("%{}%", name_filter))
                    .bind(is_active)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.db.pool)
                    .await?
                } else {
                    sqlx::query_as::<_, User>(
                        r#"
                        SELECT id, email, name, is_active, created_at, updated_at
                        FROM users
                        WHERE email ILIKE $1 AND name ILIKE $2
                        ORDER BY created_at DESC
                        LIMIT $3 OFFSET $4
                        "#
                    )
                    .bind(format!("%{}%", email_filter))
                    .bind(format!("%{}%", name_filter))
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.db.pool)
                    .await?
                }
            }
            None => {
                sqlx::query_as::<_, User>(
                    r#"
                    SELECT id, email, name, is_active, created_at, updated_at
                    FROM users
                    ORDER BY created_at DESC
                    LIMIT $1 OFFSET $2
                    "#
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.db.pool)
                .await?
            }
        };

        Ok(users)
    }

    pub async fn update_user(&self, id: Uuid, input: UpdateUserInput) -> AppResult<User> {
        input.validate().map_err(|e| AppError::Validation(e.to_string()))?;

        // Check if user exists first
        let existing_user = self.get_user_by_id(id).await?;

        let email = input.email.unwrap_or(existing_user.email);
        let name = input.name.unwrap_or(existing_user.name);
        let is_active = input.is_active.unwrap_or(existing_user.is_active);
        let now = Utc::now();

        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users 
            SET email = $1, name = $2, is_active = $3, updated_at = $4
            WHERE id = $5
            RETURNING id, email, name, is_active, created_at, updated_at
            "#
        )
        .bind(email)
        .bind(name)
        .bind(is_active)
        .bind(now)
        .bind(id)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(user)
    }

    pub async fn delete_user(&self, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#
        )
        .bind(id)
        .execute(&self.db.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User with id {} not found", id)));
        }

        Ok(true)
    }
}
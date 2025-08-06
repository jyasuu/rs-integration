use async_graphql::{Context, Object, Result};
use uuid::Uuid;

use crate::{
    models::user::{PaginationInput, User, UserFilter},
    services::UserService,
};

#[derive(Default)]
pub struct Query;

#[Object]
impl Query {
    async fn user(&self, ctx: &Context<'_>, id: Uuid) -> Result<User> {
        let user_service = ctx.data::<UserService>()?;
        let user = user_service.get_user_by_id(id).await?;
        Ok(user)
    }

    async fn users(
        &self,
        ctx: &Context<'_>,
        filter: Option<UserFilter>,
        pagination: Option<PaginationInput>,
    ) -> Result<Vec<User>> {
        let user_service = ctx.data::<UserService>()?;
        let users = user_service
            .get_users(filter, pagination.unwrap_or_default())
            .await?;
        Ok(users)
    }
}
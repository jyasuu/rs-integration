use async_graphql::{Context, Object, Result};
use uuid::Uuid;

use crate::{
    models::user::{CreateUserInput, UpdateUserInput, User},
    services::UserService,
};

#[derive(Default)]
pub struct Mutation;

#[Object]
impl Mutation {
    async fn create_user(&self, ctx: &Context<'_>, input: CreateUserInput) -> Result<User> {
        let user_service = ctx.data::<UserService>()?;
        let user = user_service.create_user(input).await?;
        Ok(user)
    }

    async fn update_user(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: UpdateUserInput,
    ) -> Result<User> {
        let user_service = ctx.data::<UserService>()?;
        let user = user_service.update_user(id, input).await?;
        Ok(user)
    }

    async fn delete_user(&self, ctx: &Context<'_>, id: Uuid) -> Result<bool> {
        let user_service = ctx.data::<UserService>()?;
        let result = user_service.delete_user(id).await?;
        Ok(result)
    }
}
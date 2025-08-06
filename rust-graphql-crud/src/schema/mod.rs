pub mod mutation;
pub mod query;

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, Schema,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};

use crate::{database::Database, services::UserService};
use mutation::Mutation;
use query::Query;

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

pub async fn build_schema(database: Database) -> AppSchema {
    let user_service = UserService::new(database);

    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .data(user_service)
        .finish()
}

pub async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

pub async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}
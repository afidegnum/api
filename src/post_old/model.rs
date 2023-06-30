use serde::{Deserialize, Serialize};
use tokio_pg_mapper_derive::PostgresMapper;
use utoipa::{IntoParams, ToResponse, ToSchema};

// extern crate chrono;
//use chrono::prelude::*;
//use chrono::{DateTime, Duration, Utc};
use chrono::Utc;
//To be added based on special query

#[derive(
    Serialize, Debug, ToSchema, Clone, ToResponse, IntoParams, Deserialize, PostgresMapper, Default,
)]
#[schema(title = "Posts")]
#[schema(example = json!({"class": "post inline"}))]
#[response(description = "Latest Post")]
#[pg_mapper(table = "posts")]
pub struct Post {
    pub id: i32,
    #[schema(example = json!({"widget": "fieldset", "class": "fielset px-2"}))]
    #[schema(example = json!({"widget": "text", "class": "text-form px-2"}))]
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub content: String,
    #[schema(value_type = String)]
    pub submitted_date: chrono::DateTime<Utc>,
    #[schema(value_type = String)]
    pub modified_date: chrono::DateTime<Utc>,
}

#[derive(
    Serialize, Debug, ToSchema, Clone, ToResponse, IntoParams, Deserialize, PostgresMapper, Default,
)]
#[schema(title = "New Post")]
#[schema(example = json!({"class": "form inline"}))]
#[response(description = "Create a new post")]
#[pg_mapper(table = "posts")]
pub struct CreatePost {
    pub title: String,
    #[schema(example = json!({"widget": "fieldset", "class": "fielset px-2"}))]
    #[schema(example = json!({"widget": "text", "class": "text-form px-2"}))]
    pub slug: String,
    pub summary: String,
    pub content: String,
}

use crate::post::db;
use crate::post::model::{CreatePost, Post};
// use crate::{LogApiKey, RequireApiKey};
use actix_web::web::put;
use futures::future::{ready, Ready};
use serde::{Deserialize, Serialize};
use std::io;

use actix_web::{
    delete, error, get, patch, post, put, web, Error, HttpRequest, HttpResponse, Responder, Result,
};
use deadpool_postgres::{Client, Pool};
use io::ErrorKind::NotFound;

use derive_more::{Display, Error};

/// Get list of posts.
///
/// List postss from in-memory post store.
///
/// One could call the api endpoint with following curl.
/// ```text
/// curl localhost:8080/posts
/// ```
#[utoipa::path(
    responses(
        (status = 200, description = "List current todo items", body = [Post]),
    )
)]
#[get("/")]
pub async fn list_post(db_pool: web::Data<Pool>) -> impl Responder {
    let client: Client = db_pool
        .get()
        .await
        .expect("Error connecting to the database");

    let result = db::post_list(&client).await;

    match result {
        Ok(object) => HttpResponse::Ok().json(object),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

/// Todo endpoint error responses
#[derive(Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub(crate) enum ErrorResponse {
    /// When Todo is not found by search term.
    NotFound(String),
    /// When there is a conflict storing a new todo.
    Conflict(String),
    /// When todo endpoint was called without correct credentials
    Unauthorized(String),
}

/// Create new Todo to shared in-memory storage.
///
/// Post a new `Todo` in request body as json to store it. Api will return
/// created `Todo` on success or `ErrorResponse::Conflict` if todo with same id already exists.
///
/// One could call the api with.
/// ```text
/// curl localhost:8080/todo -d '{"id": 1, "value": "Buy movie ticket", "checked": false}'
/// ```
#[utoipa::path(
    request_body = CreatePost,
    responses(
        (status = 201, description = "Post created successfully", body = Post),
        (status = 409, description = "Post with id already exists", body = ErrorResponse, example = json!(ErrorResponse::Conflict(String::from("id = 1"))))
    )
)]
#[post("/")]
pub async fn add_post(
    local_object: web::Json<CreatePost>,
    db_pool: web::Data<Pool>,
) -> impl Responder {
    let client: Client = db_pool
        .get()
        .await
        .expect("Error connecting to the database");

    let result = db::post_add(&client, local_object.clone()).await;
    match result {
        Ok(object) => HttpResponse::Ok().json(object),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

/// Get Todo by given todo id.
///
/// Return found `Todo` with status 200 or 404 not found if `Todo` is not found from shared in-memory storage.
#[utoipa::path(
    responses(
        (status = 200, description = "Post found from storage", body = Post),
        (status = 404, description = "Post not found by id", body = ErrorResponse, example = json!(ErrorResponse::NotFound(String::from("id = 1"))))
    ),
    params(
        ("id", description = "Unique Post ID")
    )
)]
#[get("/{id}")]
pub async fn get_post(id_path: web::Path<(i32,)>, db_pool: web::Data<Pool>) -> impl Responder {
    let client: Client = db_pool
        .get()
        .await
        .expect("Error connecting to the database");
    let path = id_path.into_inner();
    let result = db::post_id(&client, path.0).await;

    match result {
        Ok(object) => HttpResponse::Ok().json(object),
        Err(ref e) if e.kind() == NotFound => HttpResponse::NotFound().into(),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

/// Search todos Query
#[derive(Deserialize, Debug, utoipa::IntoParams)]
pub struct SearchPost {
    /// Content that should be found from Todo's value field
    pub value: String,
}

/// Search Todos with by value
///
/// Perform search from `Todo`s present in in-memory storage by matching Todo's value to
/// value provided as query parameter. Returns 200 and matching `Todo` items.
#[utoipa::path(
    params(
        SearchPost
    ),
    responses(
        (status = 200, description = "Search Todos did not result error", body = [Post]),
    )
)]
#[get("/type/{id}")]
pub async fn search_post(
    query: web::Query<SearchPost>,
    db_pool: web::Data<Pool>,
) -> impl Responder {
    let client: Client = db_pool
        .get()
        .await
        .expect("Error connecting to the database");

    // let path = query.value.into_inner();

    let result = db::post_search(&client, query.value.to_string()).await;

    match result {
        Ok(object) => HttpResponse::Ok().json(object),
        Err(ref e) if e.kind() == NotFound => HttpResponse::NotFound().into(),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

/// Delete Todo by given path variable id.
///
/// This endpoint needs `api_key` authentication in order to call. Api key can be found from README.md.
///
/// Api will delete todo from shared in-memory storage by the provided id and return success 200.
/// If storage does not contain `Todo` with given id 404 not found will be returned.
#[utoipa::path(
    responses(
        (status = 200, description = "Todo deleted successfully"),
        (status = 401, description = "Unauthorized to delete Todo", body = ErrorResponse, example = json!(ErrorResponse::Unauthorized(String::from("missing api key")))),
        (status = 404, description = "Todo not found by id", body = ErrorResponse, example = json!(ErrorResponse::NotFound(String::from("id = 1"))))
    ),
    params(
        ("id", description = "Unique storage id of Todo")
    ),
    security(
        ("api_key" = [])
    )
)]
#[delete("/{id}"/*, wrap = "RequireApiKey" */)]
pub async fn delete_post(id_path: web::Path<(i32,)>, db_pool: web::Data<Pool>) -> impl Responder {
    let client: Client = db_pool
        .get()
        .await
        .expect("Error connecting to the database");

    let path = id_path.into_inner();

    let result = db::post_delete(&client, path.0).await;

    match result {
        Ok(object) => HttpResponse::Ok().json(object),
        Err(ref e) if e.kind() == NotFound => HttpResponse::NotFound().into(),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

/// Update Todo with given id.
///
/// This endpoint supports optional authentication.
///
/// Tries to update `Todo` by given id as path variable. If todo is found by id values are
/// updated according `TodoUpdateRequest` and updated `Todo` is returned with status 200.
/// If todo is not found then 404 not found is returned.
#[utoipa::path(
    request_body = CreatePost,
    responses(
        (status = 200, description = "Todo updated successfully", body = Post),
        (status = 404, description = "Todo not found by id", body = ErrorResponse, example = json!(ErrorResponse::NotFound(String::from("id = 1"))))
    ),
    params(
        ("id", description = "Unique storage id of Post")
    ),
    security(
        (),
        ("api_key" = [])
    )
)]
#[put("/{id}"/*, wrap = "LogApiKey" */)]
pub async fn update_post(
    id_path: web::Path<(i32,)>,
    local_object: web::Json<CreatePost>,
    db_pool: web::Data<Pool>,
) -> impl Responder {
    let client: Client = db_pool
        .get()
        .await
        .expect("Error connecting to the database");

    let path = id_path.into_inner();

    let result = db::post_update(&client, path.0, local_object.clone()).await;

    match result {
        Ok(object) => HttpResponse::Ok().json(object),
        Err(ref e) if e.kind() == NotFound => HttpResponse::NotFound().into(),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_post);
    cfg.service(add_post);
    cfg.service(update_post);
    cfg.service(get_post);
    cfg.service(delete_post);
    /*
    cfg.service(list_graph);
    */
}

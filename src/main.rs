use actix_cors::Cors;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use futures::future::LocalBoxFuture;
use serde::Serialize;
// use category::ErrorResponse;
use std::{
    error::Error,
    future::{self, Ready},
    net::Ipv4Addr,
};

pub mod auth;
pub mod category;
pub mod configs;
pub mod errors;
pub mod mail;
pub mod posts;
pub mod posts_tags;
pub mod tags;
use deadpool_postgres::{Runtime, Pool};
use dotenv::dotenv;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::configs::Config;

const API_KEY_NAME: &str = "todo_apikey";
const API_KEY: &str = "utoipa-rocks";

// // API KEY
// /// Require api key middleware will actually require valid api key
// struct RequireApiKey;

// impl<S> Transform<S, ServiceRequest> for RequireApiKey
// where
//     S: Service<
//         ServiceRequest,
//         Response = ServiceResponse<actix_web::body::BoxBody>,
//         Error = actix_web::Error,
//     >,
//     S::Future: 'static,
// {
//     type Response = ServiceResponse<actix_web::body::BoxBody>;
//     type Error = actix_web::Error;
//     type Transform = ApiKeyMiddleware<S>;
//     type InitError = ();
//     type Future = Ready<Result<Self::Transform, Self::InitError>>;

//     fn new_transform(&self, service: S) -> Self::Future {
//         future::ready(Ok(ApiKeyMiddleware {
//             service,
//             log_only: false,
//         }))
//     }
// }

// /// Log api key middleware only logs about missing or invalid api keys
// struct LogApiKey;

// impl<S> Transform<S, ServiceRequest> for LogApiKey
// where
//     S: Service<
//         ServiceRequest,
//         Response = ServiceResponse<actix_web::body::BoxBody>,
//         Error = actix_web::Error,
//     >,
//     S::Future: 'static,
// {
//     type Response = ServiceResponse<actix_web::body::BoxBody>;
//     type Error = actix_web::Error;
//     type Transform = ApiKeyMiddleware<S>;
//     type InitError = ();
//     type Future = Ready<Result<Self::Transform, Self::InitError>>;

//     fn new_transform(&self, service: S) -> Self::Future {
//         future::ready(Ok(ApiKeyMiddleware {
//             service,
//             log_only: true,
//         }))
//     }
// }

// struct ApiKeyMiddleware<S> {
//     service: S,
//     log_only: bool,
// }

// impl<S> Service<ServiceRequest> for ApiKeyMiddleware<S>
// where
//     S: Service<
//         ServiceRequest,
//         Response = ServiceResponse<actix_web::body::BoxBody>,
//         Error = actix_web::Error,
//     >,
//     S::Future: 'static,
// {
//     type Response = ServiceResponse<actix_web::body::BoxBody>;
//     type Error = actix_web::Error;
//     type Future = LocalBoxFuture<'static, Result<Self::Response, actix_web::Error>>;

//     fn poll_ready(
//         &self,
//         ctx: &mut core::task::Context<'_>,
//     ) -> std::task::Poll<Result<(), Self::Error>> {
//         self.service.poll_ready(ctx)
//     }

//     fn call(&self, req: ServiceRequest) -> Self::Future {
//         let response = |req: ServiceRequest, response: HttpResponse| -> Self::Future {
//             Box::pin(async { Ok(req.into_response(response)) })
//         };

//         match req.headers().get(API_KEY_NAME) {
//             Some(key) if key != API_KEY => {
//                 if self.log_only {
//                     log::debug!("Incorrect api api provided!!!")
//                 } else {
//                     return response(
//                         req,
//                         HttpResponse::Unauthorized().json(ErrorResponse::Unauthorized(
//                             String::from("incorrect api key"),
//                         )),
//                     );
//                 }
//             }
//             None => {
//                 if self.log_only {
//                     log::debug!("Missing api key!!!")
//                 } else {
//                     return response(
//                         req,
//                         HttpResponse::Unauthorized()
//                             .json(ErrorResponse::Unauthorized(String::from("missing api key"))),
//                     );
//                 }
//             }
//             _ => (), // just passthrough
//         }

//         if self.log_only {
//             log::debug!("Performing operation")
//         }

//         let future = self.service.call(req);

//         Box::pin(async move {
//             let response = future.await?;

//             Ok(response)
//         })
//     }
// }
// // API KEY
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_server=debug,actix_web=debug");
    std::env::set_var("RUST_BACKTRACE", "full");
    env_logger::init();

    #[derive(OpenApi)]
    #[openapi(
        info(title = "authentication middleware"),
        paths(
            auth::register_user,
            auth::process_login,
            category::category,
            category::add_category,
            category::update_category,
            category::get_category,
            category::delete_category,
            tags::tags,
            tags::add_tags,
            tags::update_tags,
            tags::get_tags,
            tags::delete_tags,
            posts::posts,
            posts::add_posts,
            posts::update_posts,
            posts::get_posts,
            posts::delete_posts,
        ),
        components(
            schemas(auth::CreateUser, errors::ServiceError, category::Category, category::CreateCategory, tags::Tags, tags::CreateTags, posts::Post, posts::CreatePost)
        )
           //  ,
        // tags(
        //     (name = "Auth", description = "Authentication Mechanism")
        // )
    )]
    #[derive(Serialize, Clone, Debug)]
    struct ApiDoc;
    let openapi = ApiDoc::openapi();

    #[derive(Serialize, Debug)]
    struct ApiPath {
        api: ApiDoc,
    }

    let config = Config::from_env().unwrap();
    // let config = configs::Config::new();
    let bind_addr = format!("{}:{}", config.srv_cnf.host, config.srv_cnf.port);
    println!(
        "Starting server at http://{}:{}",
        config.srv_cnf.host, config.srv_cnf.port
    );

    let pool = config
        .pg
        .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
        .unwrap();

    let server = HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Logger::new("%% |Origin: %a |Time: %t |Method: %r |Status: %s |Size: %b |ReqTime: %D |RemoteIP: %{r}a |Request URL: %U %{User-Agent}i"))
            .wrap(cors)
            // .service(web::scope("/categories").configure(category::init_routes))
            .service(web::scope("/auth").configure(auth::init_routes))
            .service(web::scope("/posts").configure(posts::init_routes))
            .service(web::scope("/categories").configure(category::init_routes))
            // .service(web::scope("/posts_tags").configure(posts_tags::init_routes))
            .service(web::scope("/tags").configure(tags::init_routes))
            .service(
                web::resource("/api.json").route(web::get().to(|oapi: web::Data<Pool>| async move {
                    // let json_api = oapi.as_ref().api.clone();
                    // let json_api = openapi.get_ref().openapi().clone(); // Access openapi from app data

                    let json_api = ApiDoc::openapi();

                     HttpResponse::Ok().json(json_api.clone())
                })),
            )
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
    })
    .bind(bind_addr)?
    // .bind_uds("/tmp/auth-uds.socket")?
    .run();

    server.await
}

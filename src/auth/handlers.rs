use futures::future::{ready, Ready};
// use paperclip::actix::api_v2_operation;
use actix_identity::Identity;
use rand::Rng;
// use schemars::schema_for;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::io;

use actix_web::{
    delete, error, get, patch, post, web, Error, FromRequest, HttpRequest, HttpResponse, Responder,
    Result,
};
use deadpool_postgres::{Client, Pool};
use io::ErrorKind::NotFound;

// use derive_more::{Display, Error};

//use crate::components::forms;
//us e crate::config;
//use crate::custom_error::CustomError;
//use crate::layouts;

use actix_web::http;

//use sqlx::PgPool;
use std::borrow::Cow;
use std::default::Default;

use crate::auth::db;
use crate::auth::model::{CreateUser, Session, SessionAdd};
use crate::configs;
use crate::mail::model::Message;

// use crate::auth::{db, UISchemaField, UserUISchema};
use crate::errors::ServiceError;
use crate::mail::send_email;

use super::{
    add_session, delete_session, encryption, find_user_by_mail, find_user_by_session,
    find_user_mail_by_id, hex_to_bytes, session_otp_set_attempts, session_otp_update_confirm_true,
    session_otp_update_true, Otp,
};
// use validator::{Validate, ValidationError, ValidationErrors};

/// Create User | Top

/// Create an Account
#[utoipa::path(
    context_path = "/auth",
    request_body(content = CreateUser, description = "Create User", content_type = "application/json",  example = json!({"id": 1, "name": "bob the cat"})),
    responses(
        (status = 201, description = "User created successfully", body = CreateUser),
        (status = 409, description = "User with id already exists", body = ErrorResponse, example = json!(crate::auth::ErrorResponse::Conflict(String::from("id = 1"))))
    )
)]
#[post("/")]
pub async fn register_user(
    db_pool: web::Data<Pool>,
    jsonusr: web::Json<CreateUser>,
) -> impl Responder {
    let client: Client = db_pool
        .get()
        .await
        .expect("Error connecting to the database");

    let usr = CreateUser {
        email: jsonusr.email.clone(),
        hashed_password: jsonusr.hashed_password.clone(),
    };
    let result = db::add_user(&client, usr).await;

    match result {
        Ok(object) => HttpResponse::Ok().json(object),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

pub async fn session_create(
    pool: web::Data<Pool>,
    identity: Identity,
    user_id: i32,
    master_key_hash: Option<String>,
) -> Result<(), ServiceError> {
    // We generate and OTP code and encrypt it.
    // Encryption helps secure against an attacker who has read only access to the database

    let config = configs::Config::from_env().unwrap();
    let hex = &config.srv_cnf.secret_key;
    let hex = encryption::hex_to_bytes(&hex).expect("SECRET_KEY could not parse");

    let mut rng = rand::thread_rng();
    let otp_code: u32 = rng.gen_range(10000..99999);
    let otp_encrypted =
        encryption::encrypt(&format!("{}", otp_code), &format!("{}", user_id), &hex)?;

    // Create a random session verifier
    let random_bytes = rand::thread_rng().gen::<[u8; 32]>();
    let mut hasher = Sha256::new();
    // Hash it to avoid exposing it in the database.
    hasher.update(random_bytes);
    let hex_hashed_session_verifier = hex::encode(hasher.finalize());

    // let session = sqlx::query_as::<_, InsertedSession>(
    //     "
    //         INSERT INTO sessions (user_id, session_verifier, otp_code_encrypted)
    //         VALUES($1, $2, $3) RETURNING id
    //     ",
    // )
    // .bind(user_id)
    // .bind(hex_hashed_session_verifier)
    // .bind(otp_encrypted)
    // .fetch_one(pool.get_ref())
    // .await?;

    let sess = SessionAdd {
        user_id,
        session_verifier: hex_hashed_session_verifier,
        otp_code_encr: otp_encrypted,
    };

    let client: Client = pool.get().await.expect("Error connecting to the database");

    let sess_res = add_session(&client, sess).await?;

    let session = Session {
        session_id: sess_res.id,
        session_verifier: hex::encode(random_bytes),
        master_key_hash,
    };

    let serialized =
        serde_json::to_string(&session).map_err(|e| ServiceError::FaultySetup(e.to_string()))?;

    // identity.remember(serialized);

    Ok(())
}

/// Login | Top
///
/// Login your account
#[utoipa::path(
    context_path = "/auth",
    request_body = CreateUser,
    responses(
        (status = 201, description = "User logged successfully", body = CreateUser),
        (status = 409, description = "Authentication Failure", body = ErrorResponse, example = json!(crate::auth::ErrorResponse::Conflict(String::from("id = 1"))))
    )
)]
#[post("/login")]
pub async fn process_login(
    pool: web::Data<Pool>,
    identity: Identity,
    login: web::Json<CreateUser>,
) -> Result<HttpResponse, ServiceError> {
    let client: Client = pool.get().await.expect("Error connecting to the database");

    let email = &login.email;
    let config = configs::Config::from_env().unwrap();

    match find_user_by_mail(&client, email.to_string()).await {
        Ok(user) => {
            if encryption::verify_hash(
                &login.hashed_password,
                &user.hashed_password,
                config.srv_cnf.bcrypt_or_argon,
            )
            .await?
            {
                session_create(pool, identity, user.id.clone(), None).await?;
                return Ok(HttpResponse::Accepted().finish());
            } else {
                return Ok(HttpResponse::Unauthorized().json("Authentication failure"));
            }
        }
        Err(_) => {
            return Ok(HttpResponse::NotFound().json("Account does not exist"));
        }
    }
}

impl FromRequest for Session {
    type Error = ServiceError;
    type Future = Ready<Result<Session, ServiceError>>;

    fn from_request(req: &HttpRequest, pl: &mut actix_web::dev::Payload) -> Self::Future {
        if let Ok(identity) = Identity::from_request(req, pl).into_inner() {
            if let Some(session_id_and_verifier) = identity.id().ok() {
                let parsed_cookie: Result<Session, serde_json::Error> =
                    serde_json::from_str(&session_id_and_verifier);
                if let Ok(parsed_cookie) = parsed_cookie {
                    let mut hasher = Sha256::new();
                    let bytes = hex::decode(&parsed_cookie.session_verifier);
                    if let Ok(bytes) = bytes {
                        hasher.update(bytes);
                        let hex_hashed_session_verifier = hex::encode(hasher.finalize());
                        return futures::future::ok(Session {
                            session_id: parsed_cookie.session_id,
                            session_verifier: hex_hashed_session_verifier,
                            master_key_hash: parsed_cookie.master_key_hash,
                        });
                    }
                }
            }
        }
        futures::future::err(ServiceError::Unauthorized)
    }
}

pub async fn logout(
    id: Identity,
    config: web::Data<config::Config>,
    session: Option<Session>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ServiceError> {
    let client: Client = pool.get().await.expect("Error connecting to the database");

    if let Some(session) = session {
        delete_session(&client, session);
    }

    id.logout();

    return Ok(HttpResponse::Ok().json("Logout Successfully"));
}

pub async fn email_otp(
    session: Option<Session>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, ServiceError> {
    let client: Client = pool.get().await.expect("Error connecting to the database");
    let config = configs::Config::from_env().unwrap();
    let secret = hex_to_bytes(&config.srv_cnf.secret_key).expect("SECRET_KEY could not parse");

    if let Some(session) = session {
        if let Some(user_session) = find_user_by_session(&client, session).await {
            if !user_session.otp_code_sent {
                session_otp_update_true(&client, user_session.id);

                let mail_id = find_user_mail_by_id(&client, user_session.id).await;
                let otp_code = encryption::decrypt(
                    &user_session.otp_code_encrypted,
                    &format!("{}", user_session.user_id),
                    &secret,
                )?;
                if let Ok(db_user) = mail_id {
                    let body = format!(" <p>Your Confirmation code is : {} </p>", otp_code);
                    let message = Message {
                        email: db_user.email,
                        subject: "Your Confirmation Code".to_owned(),
                        msg: body,
                    };
                    send_email(message);
                } else if user_session.user_id == config.srv_cnf.user_invalid_id {
                    // Looks like the an attempt to register a duplicate user
                    // There may be a timing attack here.
                }

                // if let Some(smtp_config) = &config.srv_cnf {

                // }
            }

            // let body = OtpPage {
            //     hcaptcha: user_session.otp_code_attempts > 0,
            //     hcaptcha_config: &config.hcaptcha_config,
            //     errors: &ValidationErrors::default(),
            // };

            // return Ok(layouts::session_layout(
            //     "Confirmation Code",
            //     &body.to_string(),
            //     config.hcaptcha_config.is_some(),
            // ));
        }
    }

    // We shouldn't be here without a session. Go to sign in.
    // Ok(HttpResponse::SeeOther()
    //     .append_header((http::header::LOCATION, crate::SIGN_IN_URL))
    //     .finish())
    Ok(HttpResponse::Accepted().finish())
}

pub async fn confirm_otp(
    pool: web::Data<Pool>,
    session: Option<Session>,
    otp: web::Json<Otp>,
) -> Result<HttpResponse, ServiceError> {
    let config = configs::Config::from_env().unwrap();
    let client: Client = pool.get().await.expect("Error connecting to the database");
    let secret = hex_to_bytes(&config.srv_cnf.secret_key).expect("SECRET_KEY could not parse");

    if let Some(session) = session {
        if let Some(user_session) = find_user_by_session(&client, session).await {
            // If we have more than 1 attempt we need to apply the Hcaptcha
            if user_session.otp_code_attempts > 0 {
                // The hCaptcha was invalid send them back.
                return Ok(HttpResponse::Unauthorized().json("OTP failure: Retry"));
                // Ok(HttpResponse::SeeOther()
                // .append_header((http::header::LOCATION, crate::EMAIL_OTP_URL))
                // .finish());
            }

            // Brute force detection
            if user_session.otp_code_attempts > config.srv_cnf.max_otp_attempts {
                // In the case of what looks like a brute force, log them out.
                return Ok(HttpResponse::Unauthorized().json("Authentication failure"));
                // Ok(HttpResponse::SeeOther()
                // .append_header((http::header::LOCATION, crate::SIGN_OUT_URL))
                // .finish());
            }

            let otp_code = encryption::decrypt(
                &user_session.otp_code_encrypted,
                &format!("{}", user_session.user_id),
                &secret,
            )?;

            if otp_code == otp.code {
                session_otp_update_confirm_true(&client, user_session.id).await?;

                // if config.auth_type == crate::config::AuthType::Encrypted {
                //     return Ok(HttpResponse::SeeOther()
                //         .append_header((http::header::LOCATION, crate::DECRYPT_MASTER_KEY_URL))
                //         .finish());
                // }

                return Ok(HttpResponse::Accepted().json("Accepted"));
            } else {
                // sqlx::query(
                //     "
                //     UPDATE
                //         sessions
                //     SET
                //         otp_code_attempts = otp_code_attempts + 1
                //     WHERE
                //         id = $1
                //     ",
                // )
                // .bind(user_session.id)
                // .execute(pool.get_ref())
                // .await?;
                session_otp_set_attempts(&client, user_session.id);

                return Ok(HttpResponse::Unauthorized().json("Too Much Attempts "));

                // return Ok(HttpResponse::SeeOther()
                //     .append_header((http::header::LOCATION, crate::EMAIL_OTP_URL))
                //     .finish());
            }
        }
    }
    return Ok(HttpResponse::Unauthorized().json("Please Login"));

    // return Ok(HttpResponse::SeeOther()
    //     .append_header((http::header::LOCATION, crate::SIGN_IN_URL))
    //     .finish());
}
// pub async fn process_login(
//     config: web::Data<config::Config>,
//     pool: web::Data<Pool>,
//     identity: Identity,
//     login: web::Json<CreateUser>,
// ) -> Result<HttpResponse, ServiceError> {
//     // let mut validation_errors = ValidationErrors::default();
//     let client: Client = pool.get().await.expect("Error connecting to the database");

//     let email = &login.email;

//     let user = find_user(&client, email.to_string()).await;
//     let config = configs::Config::from_env().unwrap();

//     match user {
//         Some(usr) {
//             let valid = encryption::verify_hash(&login.hashed_password, &login.hashed_password, config.srv_cnf.bcrypt_or_argon).await;
//             if valid {
//                 session_create(pool, identity, user_id, master_key_hash).await;
//                 return Ok(HttpResponse::Accepted())
//             }
//         }
//     }
// }

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register_user);
    cfg.service(process_login);
}

use actix_web::{HttpResponse, ResponseError};
use deadpool_postgres::PoolError;

use tokio_pg_mapper::Error as PGMError;
use tokio_postgres::error::Error as PGError;

/*
#[derive(Display, From, Debug)]
pub enum MyError {
    NotFound,
    PGError(PGError),
    PGMError(PGMError),
    PoolError(PoolError),
}
// impl std::error::Error for MyError {}
impl ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            MyError::NotFound => HttpResponse::NotFound().finish(),
            MyError::PoolError(ref err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}
*/

// use derive_more::Display;
// use derive_more::{Add, Display, From, Into};
// use diesel::result::{DatabaseErrorKind, Error as DBError};
// use actix_web::error::BlockingError;
use actix_web::Error as ActixError;
use core::fmt;
use deadpool_postgres::tokio_postgres::Error;
use utoipa::ToSchema;

// use std::fmt::Display;
// use openid::error::Error as OpenIDError;
// use postgres_query::extract::Error as PostExtractError;

// use lettre::error::Error as EmailError;
// use lettre::transport::smtp::Error as SmtpError;
// use lettre::smtp::error::Error as SmtpError;
// use lettre_email::error::Error as EmailError;
// use std::convert::{From, Into};
use anyhow::Error as AnyError;
use envy::Error as EnvyError;

// use std::convert::From;
use std::error::Error as StdError;
use std::io::Error as IoError;
// use uuid::Error as UuidError;

#[derive(Debug, ToSchema)]
pub enum ServiceError {
    DuplicateValue(String),
    BadRequest(String),
    BadId,
    NotFound(String),
    ProcessError(String),
    PGError(PGError),
    PGMError(PGMError),
    PoolError(PoolError),
    InternalServerError(String),
    // Unauthorized(String),
    // Smtp(SmtpError),
    // Mail(EmailError),
    PostgressError(String),
    AuthenticationError(String),
    GenericError(String),
    BlockingError(String),
    Conflict(String),
    FaultySetup(String),
    DatabaseError(String),
    Unauthorized,
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::BadId => HttpResponse::BadRequest().json("Invalid ID"),

            ServiceError::NotFound(ref message) => HttpResponse::NotFound().json(message),
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(message),
            ServiceError::Conflict(ref message) => HttpResponse::BadRequest().json(message),

            ServiceError::ProcessError(ref message) => {
                HttpResponse::InternalServerError().json(message)
            }

            ServiceError::PostgressError(ref message) => {
                HttpResponse::InternalServerError().json(message)
            }

            ServiceError::InternalServerError(ref message) => {
                HttpResponse::InternalServerError().json(message)
            }

            // ServiceError::Unauthorized(ref message) => HttpResponse::Unauthorized().json(message),
            ServiceError::AuthenticationError(ref message) => {
                HttpResponse::Unauthorized().json(message)
            }
            // ServiceError::Smtp(ref message) => {
            //     HttpResponse::Unauthorized().json(message.to_string())
            // }
            // ServiceError::Mail(ref message) => {
            //     HttpResponse::Unauthorized().json(message.to_string())
            // }
            ServiceError::DuplicateValue(ref message) => HttpResponse::BadRequest().json(message),

            ServiceError::GenericError(ref message) => HttpResponse::BadRequest().json(message),
            ServiceError::BlockingError(ref message) => HttpResponse::BadRequest().json(message),
            ServiceError::PGError(_) => HttpResponse::BadRequest().json("PG Error"),
            ServiceError::PGMError(_) => HttpResponse::BadRequest().json("PG Mapper Error"),
            ServiceError::PoolError(_) => HttpResponse::BadRequest().json("Pool Error"),
            ServiceError::FaultySetup(err) => {
                HttpResponse::InternalServerError().json(err.to_string())
            }
            ServiceError::DatabaseError(err) => {
                HttpResponse::InternalServerError().json(err.to_string())
            }
            ServiceError::Unauthorized => HttpResponse::Unauthorized().json("UnAuthorized"),
        }
    }
}

// impl From<UuidError> for ServiceError {
//     fn from(_: UuidError) -> ServiceError {
//         ServiceError::BadId
//     }
// }

impl From<EnvyError> for ServiceError {
    fn from(error: EnvyError) -> ServiceError {
        ServiceError::GenericError(error.to_string())
    }
}

impl From<AnyError> for ServiceError {
    fn from(error: AnyError) -> ServiceError {
        ServiceError::GenericError(error.to_string())
    }
}
// impl From<AnyError> for ServiceError {
//     fn from(error: AnyError) -> ServiceError {
//         ServiceError::Conflict(error.to_string())
//     }
// }

impl From<tokio_postgres::Error> for ServiceError {
    fn from(error: Error) -> ServiceError {
        ServiceError::DatabaseError(error.to_string())
    }
}

impl From<PoolError> for ServiceError {
    fn from(error: PoolError) -> ServiceError {
        ServiceError::PostgressError(error.to_string())
    }
}

// impl From<PostExtractError> for ServiceError {
//     fn from(error: PostExtractError) -> ServiceError {
//         ServiceError::PostgressError(error.to_string())
//     }
// }

impl From<ActixError> for ServiceError {
    fn from(error: ActixError) -> ServiceError {
        ServiceError::InternalServerError(error.to_string())
    }
}

// impl From<OpenIDError> for ServiceError {
//     fn from(error: OpenIDError) -> ServiceError {
//         ServiceError::GenericError(error.to_string())
//     }
// }

impl From<IoError> for ServiceError {
    fn from(error: IoError) -> ServiceError {
        ServiceError::GenericError(error.to_string())
    }
}

// rust bitcoin Allow this type to be treated like an error
// impl StdError for ServiceError {
//     fn source(&self) -> Option<&(dyn StdError + 'static)> {
//         // Generic error, underlying cause isn't tracked.
//         None
//     }
// }

// // Age using a buffered writer
// impl From<dyn StdError> for ServiceError {
//     fn from(err: std::io::Error) -> ServiceError {
//         ServiceError::FaultySetup(err.to_string())
//     }
// }

impl From<std::str::Utf8Error> for ServiceError {
    fn from(err: std::str::Utf8Error) -> ServiceError {
        ServiceError::FaultySetup(err.to_string())
    }
}

impl From<std::num::ParseIntError> for ServiceError {
    fn from(err: std::num::ParseIntError) -> ServiceError {
        ServiceError::FaultySetup(err.to_string())
    }
}

impl From<std::num::ParseFloatError> for ServiceError {
    fn from(err: std::num::ParseFloatError) -> ServiceError {
        ServiceError::FaultySetup(err.to_string())
    }
}
// impl Into<PostExtractError> for ServiceError {
//     fn into(self) -> PostExtractError {
//         PostExtractError {}
//     }
// }

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            // ServiceError::Smtp(ref err) => err.fmt(f),
            // ServiceError::Mail(ref err) => err.fmt(f),
            ServiceError::BadRequest(ref err) => err.fmt(f),
            ServiceError::InternalServerError(ref err) => err.fmt(f),
            // ServiceError::Unauthorized(ref err) => err.fmt(f),
            ServiceError::DuplicateValue(ref err) => err.fmt(f),
            ServiceError::BadId => f.write_str("bad id"),
            ServiceError::NotFound(ref err) => err.fmt(f),
            ServiceError::ProcessError(ref err) => err.fmt(f),
            ServiceError::PostgressError(ref err) => err.fmt(f),
            ServiceError::AuthenticationError(ref err) => err.fmt(f),
            ServiceError::GenericError(ref err) => err.fmt(f),
            ServiceError::BlockingError(ref err) => err.fmt(f),
            ServiceError::PGError(ref err) => err.fmt(f),
            ServiceError::PGMError(ref err) => err.fmt(f),
            ServiceError::PoolError(ref err) => err.fmt(f),
            ServiceError::Conflict(ref err) => err.fmt(f),
            ServiceError::FaultySetup(ref cause) => write!(f, "Setup Error: {}", cause),
            ServiceError::DatabaseError(ref cause) => write!(f, "Setup Error: {}", cause),
            ServiceError::Unauthorized => write!(f, "User doesn't have access"),
        }
    }
}

impl StdError for ServiceError {
    fn description(&self) -> &str {
        match *self {
            // ServiceError::Smtp(_) => "can not create email",
            // ServiceError::Mail(_) => "can not build email",
            ServiceError::BadRequest(_) => "Bad Request",
            ServiceError::InternalServerError(_) => "Internal Server Error",
            ServiceError::Unauthorized => "Unauthorized",
            ServiceError::DuplicateValue(_) => "duplicate values error",
            ServiceError::BadId => "Bad Id",
            ServiceError::NotFound(_) => "Not Found",
            ServiceError::ProcessError(_) => "Process Error",
            ServiceError::Conflict(_) => "Conflict Error",
            ServiceError::PostgressError(_) => "Error from Postgres",
            ServiceError::AuthenticationError(_) => "Auth Error",
            ServiceError::GenericError(_) => "Generic errors",
            ServiceError::BlockingError(_) => "Blocking ",
            ServiceError::PGError(_) => "Pg Error",
            ServiceError::PGMError(_) => "PGMapping Error",
            ServiceError::PoolError(_) => "Pool Error",
            ServiceError::FaultySetup(_) => "Faulty Setup Error",
            ServiceError::DatabaseError(_) => "Database Error",
        }
    }
}

// impl From<SmtpError> for ServiceError {
//     fn from(err: SmtpError) -> ServiceError {
//         ServiceError::Smtp(err)
//     }
// }

// impl From<EmailError> for ServiceError {
//     fn from(err: EmailError) -> ServiceError {
//         ServiceError::Mail(err)
//     }
// }

// // recent addition
// // non complete
// impl<T> From<BlockingError<T>> for ServiceError
// where
//     ServiceError: From<T>,
//     T: std::fmt::Debug,
// {
//     fn from(error: BlockingError<T>) -> Self {
//         match error {
//             BlockingError::Error(e) => ServiceError::from(e),
//             BlockingError::Canceled => ServiceError::internal_error(),
//         }
//     }
// }

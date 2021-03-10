use std::fmt::Debug;

use actix_web::{
    error::{BlockingError, ResponseError},
    http::StatusCode,
    web::Json,
    HttpResponse,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize, Debug)]
pub struct Success {
    pub message: &'static str,
}

pub const SUCCESS: Json<Success> = Json(Success { message: "Success" });

/// Possible rest error types
#[derive(Error, Debug)]
pub enum RestError {
    #[error("Not found")]
    NotFound,

    #[error("The performed action is forbidden")]
    Forbidden,

    #[error("Already exitsting")]
    AlreadyExists,

    #[error("Bad request")]
    BadRequest,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("User disabled")]
    UserDisabled,

    #[error("Illegal operation")]
    IllegalOperation,

    // Internal
    #[error("Unknown Internal Error")]
    Internal,

    #[error("Internal IO Error")]
    UnknownIO,
}

impl RestError {
    pub fn name(&self) -> String {
        match self {
            Self::NotFound => "NotFound".to_string(),
            Self::Forbidden => "Forbidden".to_string(),
            Self::UnknownIO => "Unknown IO".to_string(),
            Self::Internal => "Unknown".to_string(),
            Self::Unauthorized => "Unauthorized".to_string(),
            Self::AlreadyExists => "AlreadyExists".to_string(),
            Self::IllegalOperation => "IllegalOperation".to_string(),
            _ => "BadRequest".to_string(),
        }
    }
}

/// Implement ResponseError trait. Required for actix web
impl ResponseError for RestError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::AlreadyExists => StatusCode::UNPROCESSABLE_ENTITY,
            Self::IllegalOperation => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error: self.name(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}

/// Error response format. Used as json
/// encoding structure
#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    error: String,
    message: String,
}

impl From<r2d2::Error> for RestError {
    fn from(e: r2d2::Error) -> RestError {
        debug!("{:?}", e);
        RestError::Internal
    }
}

impl From<diesel::result::Error> for RestError {
    fn from(e: diesel::result::Error) -> RestError {
        debug!("{:?}", e);
        RestError::Internal
    }
}

impl From<std::io::Error> for RestError {
    fn from(e: std::io::Error) -> RestError {
        debug!("{:?}", e);
        match e.kind() {
            std::io::ErrorKind::NotFound => RestError::NotFound,
            _ => RestError::UnknownIO,
        }
    }
}

impl<T> From<BlockingError<T>> for RestError
where
    T: Into<RestError> + Debug,
{
    fn from(err: BlockingError<T>) -> Self {
        debug!("{:?}", err);
        match err {
            BlockingError::Error(err) => err.into(),
            BlockingError::Canceled => Self::Internal,
        }
    }
}

pub fn login_error(err: diesel::result::Error) -> RestError {
    match err {
        diesel::result::Error::NotFound => RestError::NotFound,
        _ => RestError::Internal,
    }
}

use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RestError {
    // IO
    #[error("Requested resource was not found")]
    NotFound,

    #[error("Internal IO Error")]
    UnknownIO,

    // Other
    #[error("The performed action is forbidden")]
    Forbidden,

    #[error("The user already exists")]
    UserExists,

    // HTTP
    #[error("Bad request")]
    BadRequest,

    // Unknown
    #[error("Unknown Internal Error")]
    Unknown,
}

impl From<std::io::Error> for RestError {
    fn from(e: std::io::Error) -> RestError {
        match e.kind() {
            std::io::ErrorKind::NotFound => RestError::NotFound,
            _ => RestError::UnknownIO,
        }
    }
}

impl RestError {
    pub fn name(&self) -> String {
        match self {
            Self::NotFound => "NotFound".to_string(),
            Self::Forbidden => "Forbidden".to_string(),
            Self::UnknownIO => "Unknown IO".to_string(),
            Self::Unknown => "Unknown".to_string(),
            _ => "BadRequest".to_string(),
        }
    }
}

impl ResponseError for RestError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::UserExists => StatusCode::BAD_REQUEST,
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

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    error: String,
    message: String,
}

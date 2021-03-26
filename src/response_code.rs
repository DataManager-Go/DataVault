use std::fmt::{Debug, Formatter};

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

pub trait AsOrigin {
    fn as_origin(&self) -> Origin;
}

#[derive(Clone, Copy, PartialEq)]
pub enum Origin {
    File,
    Files,
    LocalFile,
    Namespace,
    Tag,
    Group,
    Record,
    User,
}

impl Debug for Origin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Origin::Files => "File(s)",
                Origin::File => "File",
                Origin::LocalFile => "LocalFile",
                Origin::Namespace => "Namespace",
                Origin::Tag => "Tag",
                Origin::Group => "Group",
                Origin::Record => "Record",
                Origin::User => "User",
            }
        )
    }
}

impl AsOrigin for Origin {
    fn as_origin(&self) -> Origin {
        *self
    }
}

/// Possible rest error types
#[derive(Error, Debug, PartialEq, Clone, Copy)]
pub enum RestError {
    #[error("{0:?} not found")]
    DNotFound(Origin),

    #[error("Not found")]
    NotFound,

    #[error("File not public")]
    NotPublic,

    #[error("The performed action is forbidden")]
    Forbidden,

    #[error("Already exitsting")]
    AlreadyExists,

    #[error("Already public")]
    AlreadyPublic,

    #[error("Bad request")]
    BadRequest,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("User disabled")]
    UserDisabled,

    #[error("Multiple files matching")]
    MultipleFilesMatch,

    #[error("Illegal operation")]
    IllegalOperation,

    #[error("Not allowed")]
    NotAllowed,

    // Internal
    #[error("Unknown Internal Error")]
    Internal,

    #[error("Internal Io Error")]
    UnknownIo,

    #[error("Partial content received")]
    PartialContent,
}

impl RestError {
    pub fn name(&self) -> String {
        match self {
            Self::NotFound | Self::DNotFound(_) => "NotFound".to_string(),
            Self::Forbidden => "Forbidden".to_string(),
            Self::UnknownIo => "Unknown IO".to_string(),
            Self::Internal => "Unknown".to_string(),
            Self::Unauthorized => "Unauthorized".to_string(),
            Self::AlreadyExists => "AlreadyExists".to_string(),
            Self::IllegalOperation => "IllegalOperation".to_string(),
            Self::MultipleFilesMatch => "MultipleFilesMatch".to_string(),
            Self::NotAllowed => "NotAllowed".to_string(),
            Self::NotPublic => "NotPublic".to_string(),
            Self::PartialContent => "PartialContent".to_string(),
            _ => "BadRequest".to_string(),
        }
    }
}

/// Implement ResponseError trait. Required for actix web
impl ResponseError for RestError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::NotFound | Self::DNotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::AlreadyExists => StatusCode::UNPROCESSABLE_ENTITY,
            Self::IllegalOperation => StatusCode::UNPROCESSABLE_ENTITY,
            Self::MultipleFilesMatch => StatusCode::UNPROCESSABLE_ENTITY,
            Self::NotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            Self::AlreadyPublic => StatusCode::CONFLICT,
            Self::NotPublic => StatusCode::CONFLICT,
            Self::PartialContent => StatusCode::PARTIAL_CONTENT,
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
    fn from(i: r2d2::Error) -> RestError {
        debug!("{:?}", i);
        RestError::Internal
    }
}

impl From<diesel::result::Error> for RestError {
    fn from(i: diesel::result::Error) -> RestError {
        debug!("{:?}", i);
        RestError::Internal
    }
}

/// NotFound errors are mapped to 'NotFound' error responses.
/// This is helpful when diesel::result::NotFound is an allowed result
pub fn diesel_option<T>(i: diesel::result::Error, origin: T) -> RestError
where
    T: AsOrigin,
{
    debug!("{:?}", i);
    match i {
        diesel::result::Error::NotFound => RestError::DNotFound(origin.as_origin()),
        _ => i.into(),
    }
}

impl From<std::io::Error> for RestError {
    fn from(e: std::io::Error) -> RestError {
        debug!("{:?}", e);
        match e.kind() {
            std::io::ErrorKind::NotFound => RestError::DNotFound(Origin::LocalFile),
            _ => RestError::UnknownIo,
        }
    }
}

impl From<BlockingError> for RestError {
    fn from(_: BlockingError) -> Self {
        Self::Internal
    }
}

impl From<zip::result::ZipError> for RestError {
    fn from(z: zip::result::ZipError) -> RestError {
        debug!("{:?}", z);
        match z {
            zip::result::ZipError::Io(io) => io.into(),
            zip::result::ZipError::FileNotFound => RestError::DNotFound(Origin::LocalFile),
            _ => RestError::Internal,
        }
    }
}

pub fn login_error(err: diesel::result::Error) -> RestError {
    match err {
        diesel::result::Error::NotFound => RestError::DNotFound(Origin::User),
        _ => RestError::Internal,
    }
}

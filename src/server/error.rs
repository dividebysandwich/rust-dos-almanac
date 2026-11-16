use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not found")]
    NotFound,
    #[error("login required")]
    Unauthorized,
    #[error("not allowed")]
    Forbidden,
    /// Neither staff nor the key the game or version was added with.
    #[error("only staff, or the game profile UUID it was added with, can change this")]
    WrongKey,
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    /// A new game's name is taken: the answer carries the existing game's
    /// slug as `game`, to report on it or add a version to it.
    #[error("the game already exists as '{0}'; report on it or add a version to it instead")]
    GameExists(String),
    #[error("too many requests, try again in {0} seconds")]
    RateLimited(u64),
    #[error("database error")]
    Db(sea_orm::DbErr),
    #[error("internal error")]
    Internal(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn invalid(msg: impl Into<String>) -> Self {
        Error::Invalid(msg.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Forbidden | Error::WrongKey => StatusCode::FORBIDDEN,
            Error::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Error::Conflict(_) | Error::GameExists(_) => StatusCode::CONFLICT,
            Error::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            Error::Db(_) | Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn log(&self) {
        if let Error::Internal(e) = self {
            tracing::error!("internal: {e}");
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.log();
        let status = self.status();
        let body = match &self {
            Error::GameExists(slug) => serde_json::json!({ "error": self.to_string(), "game": slug }),
            _ => serde_json::json!({ "error": self.to_string() }),
        };
        let mut res = (status, axum::Json(body)).into_response();
        if let Error::RateLimited(secs) = self {
            res.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from(secs));
        }
        res
    }
}

// Logged where it happens: the conversion to a server function error is
// leptos's blanket one and can't do it.
impl From<sea_orm::DbErr> for Error {
    fn from(e: sea_orm::DbErr) -> Self {
        tracing::error!("database: {e}");
        Error::Db(e)
    }
}

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
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
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
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Error::Conflict(_) => StatusCode::CONFLICT,
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
        let mut res = (status, axum::Json(serde_json::json!({ "error": self.to_string() }))).into_response();
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

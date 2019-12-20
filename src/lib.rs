#[macro_use]
extern crate log;

use http::StatusCode;
use std::{error, fmt};

#[derive(Debug)]
pub struct HttpError {
    status: StatusCode,
    message: Option<String>,
    cause: Option<anyhow::Error>,
}

pub async fn recover(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(ref err) = err.find::<HttpError>() {
        error!("{}", err);
        if let Some(err) = err.cause() {
            for cause in err.chain() {
                error!("  -> {}", cause);
            }
        }

        Ok(warp::reply::with_status(
            err.message()
                .unwrap_or_else(|| err.status().canonical_reason().unwrap_or(""))
                .to_string(),
            err.status(),
        ))
    } else {
        Err(err)
    }
}


impl HttpError {
    pub fn new(status: StatusCode) -> Self {
        HttpError {
            status,
            message: None,
            cause: None,
        }
    }

    pub fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_cause<E: Into<anyhow::Error>>(mut self, cause: E) -> Self {
        self.cause = Some(cause.into());
        self
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().map(|s| &**s)
    }

    pub fn cause(&self) -> Option<&anyhow::Error> {
        self.cause.as_ref()
    }
}

pub trait ResultExt<T>: Sized {
    fn client_err(self) -> Result<T, warp::Rejection> {
        self.with_err_status(StatusCode::BAD_REQUEST)
    }

    fn server_err(self) -> Result<T, warp::Rejection> {
        self.with_err_status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn with_err_status(self, status: StatusCode) -> Result<T, warp::Rejection>;

    fn with_err_msg<F: FnOnce() -> String>(
        self,
        status: StatusCode,
        message: F,
    ) -> Result<T, warp::Rejection>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn with_err_status(self, status: StatusCode) -> Result<T, warp::Rejection> {
        self.map_err(|err| {
            warp::reject::custom(HttpError {
                status,
                message: None,
                cause: Some(err.into()),
            })
        })
    }

    fn with_err_msg<F: FnOnce() -> String>(
        self,
        status: StatusCode,
        message: F,
    ) -> Result<T, warp::Rejection> {
        self.map_err(|err| {
            warp::reject::custom(HttpError {
                status,
                message: Some(message()),
                cause: Some(err.into()),
            })
        })
    }
}

impl warp::reject::Reject for HttpError {}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fail with status {}", self.status)
    }
}

impl error::Error for HttpError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.cause.as_ref().and_then(|c| c.chain().next())
    }
}

impl From<anyhow::Error> for HttpError {
    fn from(err: anyhow::Error) -> Self {
        HttpError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: None,
            cause: Some(err),
        }
    }
}

#[macro_export]
macro_rules! not_found {
    () => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::NOT_FOUND))
    });
    ($msg:expr) => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::NOT_FOUND).with_message($msg))
    });
    ($fmt:expr, $($arg:tt)+) => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::NOT_FOUND).with_message(format!($fmt, $($arg)+)))
    });
}

#[macro_export]
macro_rules! bad_request {
    () => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::BAD_REQUEST))
    });
    ($msg:expr) => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::BAD_REQUEST).with_message($msg))
    });
    ($fmt:expr, $($arg:tt)+) => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::BAD_REQUEST).with_message(format!($fmt, $($arg)+)))
    });
}

#[macro_export]
macro_rules! forbidden {
    () => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::FORBIDDEN))
    });
    ($msg:expr) => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::FORBIDDEN).with_message($msg))
    });
    ($fmt:expr, $($arg:tt)+) => ({
        ::warp::reject::custom($crate::HttpError::new(::reqwest::StatusCode::FORBIDDEN).with_message(format!($fmt, $($arg)+)))
    });
}
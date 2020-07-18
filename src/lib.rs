#[macro_use]
extern crate log;

use http::StatusCode;
use std::error::Error as _;
use std::{error, fmt};
use warp::Rejection;

pub type BoxedError = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Debug)]
pub struct HttpError {
    status: StatusCode,
    message: Option<String>,
    source: Option<BoxedError>,
}

pub async fn recover(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(ref err) = err.find::<HttpError>() {
        error!("{}", err);
        let mut source = err.source();
        while let Some(err) = source {
            error!("  -> {}", err);
            source = err.source();
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
            source: None,
        }
    }

    pub fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_source(
        mut self,
        source: impl Into<Box<dyn error::Error + Send + Sync + 'static>>,
    ) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().map(|s| &**s)
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
    E: Into<Box<dyn error::Error + Send + Sync + 'static>>,
{
    fn with_err_status(self, status: StatusCode) -> Result<T, warp::Rejection> {
        self.map_err(|err| {
            warp::reject::custom(HttpError {
                status,
                message: None,
                source: Some(err.into()),
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
                source: Some(err.into()),
            })
        })
    }
}

pub fn status(status: StatusCode) -> HttpError {
    HttpError::new(status)
}

pub fn no_content() -> HttpError {
    HttpError::new(StatusCode::NO_CONTENT)
}

pub fn ok() -> HttpError {
    HttpError::new(StatusCode::OK)
}

pub fn internal_server_error(err: impl error::Error + Send + Sync + 'static) -> HttpError {
    HttpError::new(StatusCode::INTERNAL_SERVER_ERROR).with_source(err)
}

impl warp::reject::Reject for HttpError {}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fail with status {}", self.status)
    }
}

impl error::Error for HttpError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|err| err.as_ref() as &(dyn error::Error + 'static))
    }
}

impl From<HttpError> for Rejection {
    fn from(err: HttpError) -> Self {
        warp::reject::custom(err)
    }
}

#[macro_export]
macro_rules! not_found {
    () => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::NOT_FOUND))
    });
    ($msg:expr) => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::NOT_FOUND).with_message($msg))
    });
    ($fmt:expr, $($arg:tt)+) => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::NOT_FOUND).with_message(format!($fmt, $($arg)+)))
    });
}

#[macro_export]
macro_rules! bad_request {
    () => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::BAD_REQUEST))
    });
    ($msg:expr) => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::BAD_REQUEST).with_message($msg))
    });
    ($fmt:expr, $($arg:tt)+) => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::BAD_REQUEST).with_message(format!($fmt, $($arg)+)))
    });
}

#[macro_export]
macro_rules! forbidden {
    () => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::FORBIDDEN))
    });
    ($msg:expr) => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::FORBIDDEN).with_message($msg))
    });
    ($fmt:expr, $($arg:tt)+) => ({
        ::warp::reject::custom($crate::HttpError::new(::http::StatusCode::FORBIDDEN).with_message(format!($fmt, $($arg)+)))
    });
}

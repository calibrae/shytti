use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::fmt;

#[derive(Debug)]
pub enum Error {
    NotFound(String),
    SpawnFailed(String),
    Io(std::io::Error),
    Config(String),
    Bridge(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(id) => write!(f, "shell not found: {id}"),
            Error::SpawnFailed(msg) => write!(f, "spawn failed: {msg}"),
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Config(msg) => write!(f, "config: {msg}"),
            Error::Bridge(msg) => write!(f, "bridge: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Config(e.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match &self {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_not_found() {
        let e = Error::NotFound("abc".into());
        assert_eq!(e.to_string(), "shell not found: abc");
    }

    #[test]
    fn display_spawn_failed() {
        let e = Error::SpawnFailed("boom".into());
        assert_eq!(e.to_string(), "spawn failed: boom");
    }

    #[test]
    fn display_io() {
        let e = Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(e.to_string().starts_with("io:"));
    }

    #[test]
    fn display_config() {
        let e = Error::Config("bad toml".into());
        assert_eq!(e.to_string(), "config: bad toml");
    }

    #[test]
    fn display_bridge() {
        let e = Error::Bridge("timeout".into());
        assert_eq!(e.to_string(), "bridge: timeout");
    }

    #[test]
    fn into_response_not_found_is_404() {
        let e = Error::NotFound("x".into());
        let resp = e.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn into_response_spawn_failed_is_500() {
        let e = Error::SpawnFailed("x".into());
        let resp = e.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn into_response_io_is_500() {
        let e = Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "x"));
        let resp = e.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        let e: Error = io_err.into();
        assert!(matches!(e, Error::Io(_)));
    }

    #[test]
    fn from_toml_error() {
        let toml_err = toml::from_str::<toml::Value>("not [[[valid").unwrap_err();
        let e: Error = toml_err.into();
        assert!(matches!(e, Error::Config(_)));
    }
}

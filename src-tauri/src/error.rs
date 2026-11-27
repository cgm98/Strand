use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Error único de la capa de aplicación. Llega al frontend como
/// `{ code, message }` para que la UI pueda decidir por `code`
/// (p. ej. recargar la nota ante un `CONFLICT`).
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("error de base de datos: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("error de zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Validation(_) => "VALIDATION",
            AppError::Conflict(_) => "CONFLICT",
            AppError::Db(_) => "DB",
            AppError::Io(_) => "IO",
            AppError::Zip(_) => "ZIP",
            AppError::Internal(_) => "INTERNAL",
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// `true` si el error es una violación de un índice UNIQUE.
pub fn is_unique_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(e, _)
            if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
    )
}

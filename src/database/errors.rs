use std::fmt;

/// Error type for database operations
#[derive(Debug)]
pub enum DatabaseError {
    /// Error when connecting to the database
    ConnectionError(String),
    /// Error related to database configuration
    ConfigurationError(String),
    /// Error when executing a query
    QueryError(String),
    /// Error when converting data types
    ConversionError(String),
    /// Any other database-related error
    Other(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionError(msg) => write!(f, "Database connection error: {}", msg),
            Self::ConfigurationError(msg) => write!(f, "Database configuration error: {}", msg),
            Self::QueryError(msg) => write!(f, "Database query error: {}", msg),
            Self::ConversionError(msg) => write!(f, "Database data conversion error: {}", msg),
            Self::Other(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {}

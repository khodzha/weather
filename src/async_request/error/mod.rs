use std;
use std::fmt;
use std::error::Error;

#[derive(Debug, PartialEq)]
pub enum ApiError {
    LocationNotFound,
    Timeout,
    Other
}


impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ApiError::LocationNotFound => f.write_str("LocationNotFound"),
            ApiError::Timeout => f.write_str("Timeout"),
            ApiError::Other => f.write_str("Other"),
        }
    }
}

impl Error for ApiError {
    fn description(&self) -> &str {
        match *self {
            ApiError::LocationNotFound => "Location wasnt found",
            ApiError::Timeout => "Connection timed out",
            ApiError::Other => "Other error",
        }
    }
}

impl From<std::io::Error> for ApiError {
    fn from(_err: std::io::Error) -> Self {
        ApiError::Other
    }
}

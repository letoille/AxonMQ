use warp::http::StatusCode;

use crate::error::AxonError;
use crate::service::sparkplug_b::error::SpbError;

#[derive(Debug)]
pub enum ApiError {
    InternalError(String),
    SparkPlugBError(StatusCode, String),
}

impl warp::reject::Reject for ApiError {}

impl From<AxonError> for ApiError {
    fn from(err: AxonError) -> Self {
        use SpbError::*;
        match err {
            AxonError::SparkPlugBError(e) => match e {
                NodeNotFound | GroupNotFound => {
                    ApiError::SparkPlugBError(StatusCode::NOT_FOUND, format!("{}", e))
                }
                _ => ApiError::SparkPlugBError(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)),
            },
            _ => ApiError::InternalError(format!("{}", err)),
        }
    }
}

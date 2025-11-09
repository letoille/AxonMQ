use warp::http::StatusCode;

use super::error::ApiError;

pub(crate) async fn handle_rejection(
    err: warp::Rejection,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "ENDPOINT_NOT_FOUND".to_string();
    } else if let Some(e) = err.find::<ApiError>() {
        match e {
            ApiError::SparkPlugBError(status, msg) => {
                code = *status;
                message = msg.clone();
            }
            ApiError::InternalError(msg) => {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = msg.clone();
            }
        }
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "INTERNAL_SERVER_ERROR".to_string();
    }

    let json_response = warp::reply::json(&serde_json::json!({
        "error": message,
    }));

    Ok(warp::reply::with_status(json_response, code))
}

mod error;
mod rejection;
mod spb;

use std::net::SocketAddr;

use percent_encoding::percent_decode_str;
use warp::{Filter, http::Uri};

use crate::service::sparkplug_b::in_helper::InHelper as SpbInHelper;

use rejection::handle_rejection;
use spb::spb_routers;

pub struct RESTful {
    server: SocketAddr,
}

impl RESTful {
    pub fn new(ip: &str, port: u16) -> Result<Self, String> {
        let server = format!("{}:{}", ip, port)
            .parse::<SocketAddr>()
            .map_err(|e| format!("invalid RESTful server address: {}", e))?;

        Ok(Self { server })
    }

    pub async fn run(&self, spb_in_helper: SpbInHelper) {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec![
                "Origin",
                "Access-Control-Request-Method",
                "Access-Control-Request-Headers",
                "Refer",
                "User-Agent",
                "X-Requested-With",
            ])
            .allow_header("Content-Type")
            .allow_header("Cache-Control")
            .expose_header("Access-Control-Allow-Origin")
            .allow_methods(vec!["POST", "GET", "PUT", "DELETE"]);

        let redirect_dashboard = warp::path::end().map(|| warp::redirect(Uri::from_static("/dh")));
        let dashboard = warp::path("dh").and(warp::fs::dir("dist"));

        let api_routers = redirect_dashboard
            .or(dashboard)
            .or(spb_routers(spb_in_helper));

        let routers_with_cors = api_routers.with(cors);
        let routers_with_log = routers_with_cors.with(warp::log("axonmq::service::restful"));
        let routers_with_error_handler = routers_with_log.recover(handle_rejection);
        warp::serve(routers_with_error_handler)
            .run(self.server)
            .await;
    }
}

pub fn decode_param(param: &str) -> String {
    percent_decode_str(param).decode_utf8().unwrap().to_string()
}

pub fn with_spb_in_helper(
    spb_in_helper: SpbInHelper,
) -> impl Filter<Extract = (SpbInHelper,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || spb_in_helper.clone())
}

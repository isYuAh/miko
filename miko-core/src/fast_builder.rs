use std::convert::Infallible;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::Response;

pub struct ResponseBuilder{}
impl ResponseBuilder {
    pub fn not_found() -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
        Response::builder()
            .status(404)
            .body(box_str_resp("Not Found".to_string())).map_err(|_| unreachable!())
    }
    
    pub fn internal_server_error(err: Option<String>) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
        let msg = match err {
            Some(e) => format!("Internal Server Error: {}", e),
            None => "Internal Server Error".to_string(),
        };
        Response::builder()
            .status(500)
            .body(box_str_resp(msg)).map_err(|_| unreachable!())
    }
    
    pub fn ok(body: String) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
        Response::builder()
            .status(200)
            .body(box_str_resp(body)).map_err(|_| unreachable!())
    }

    pub fn bad_request(err: Option<String>) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
        let msg = match err {
            Some(e) => format!("Bad Request: {}", e),
            None => "Bad Request".to_string(),
        };
        Response::builder()
            .status(400)
            .body(box_str_resp(msg)).map_err(|_| unreachable!())
    }
}

fn box_str_resp (str: String) -> BoxBody<Bytes, Infallible> {
    Full::new(Bytes::from(str)).boxed()
}

pub fn map_err_to_500(err: anyhow::Error) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    ResponseBuilder::internal_server_error(Some(err.to_string()))
}
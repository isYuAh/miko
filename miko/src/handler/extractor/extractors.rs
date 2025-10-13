use crate::handler::extractor::from_request::FRPFut;
use crate::handler::extractor::path_params::PathParams;
use crate::handler::{extractor::from_request::{FRFut, FromRequest, FromRequestParts}, handler::Req};
use anyhow::anyhow;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::http::request::Parts;
use miko_core::fast_builder::boxed_err;
use mime_guess::Mime;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use hyper::http::Extensions;
use hyper::{Method, Uri};

#[derive(Debug)]
pub struct Json<T>(pub T);
pub struct Query<T>(pub T);
pub struct Path<T>(pub T);
pub struct State<T>(pub Arc<T>);
pub struct Form<T>(pub T);
#[derive(Debug)]
pub struct Multipart {
    pub fields: HashMap<String, Vec<String>>,
    pub files: HashMap<String, Vec<FileItem>>,
}
#[derive(Debug)]
pub struct FileItem {
    pub filename: String,
    pub bytes: Bytes,
    pub size: usize,
    pub content_type: Option<Mime>,
}

impl<S, T> FromRequest<S> for Json<T>
where T: DeserializeOwned + Send + Sync + 'static {
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        let _ = _state;
        Box::pin(async move {
            let body = req.body_mut().collect().await.unwrap().to_bytes();
            let json = serde_json::from_slice(&body);
            match json {
                Ok(json) => Ok(Json(json)),
                Err(err) => Err(err.into())
            }
        })
    }
}


impl<S, T> FromRequestParts<S> for Query<T>
where T: DeserializeOwned + Send + Sync + 'static {
    fn from_request_parts(req: &mut Parts, _state: Arc<S>) -> FRFut<Self> {
        let query = req.uri.query().unwrap_or("");
        let query = serde_urlencoded::from_str(query);
        Box::pin(async move {
            match query {
                Ok(query) => Ok(Query(query)),
                Err(err) => Err(err.into())
            }
        })
    }
}

impl<S, T> FromRequestParts<S> for Path<T>
where T: From<String> + Send + Sync + 'static
{
    fn from_request_parts(req: &mut Parts, _state: Arc<S>) -> FRFut<Self> {
        let pp = req.extensions.get_mut::<PathParams>().unwrap();
        if pp.0.len() < 1 {return boxed_err(anyhow!("path params not long enough"))}
        let path = pp.0.remove(0).1.clone();
        Box::pin(async move {
            Ok(Path(path.into()))
        })
    }
}

impl<S: Send + Sync + 'static> FromRequestParts<S> for State<S> {
    fn from_request_parts(_req: &mut Parts, state: Arc<S>) -> FRPFut<Self> {
        Box::pin(async move {
            Ok(State(state.clone()))
        })
    }
}

impl<S> FromRequest<S> for String {
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let body = req.body_mut().collect().await.unwrap().to_bytes();
            let string = std::str::from_utf8(&body)
                .map(|s| s.to_string())
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))?;
            Ok(string)
        })
    }
}

impl<S> FromRequest<S> for Bytes {
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let body = req.body_mut().collect().await.unwrap().to_bytes();
            Ok(body)
        })
    }
}

impl<S, T> FromRequest<S> for Form<T>
where T: DeserializeOwned + Send + Sync + 'static
{
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let body = req.body_mut().collect().await?;
            let form: T = serde_urlencoded::from_bytes(&*body.to_bytes())?;
            Ok(Form(form))
        })
    }
}

impl<S> FromRequest<S> for Multipart
{
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let mut form = HashMap::new();
            let mut files = HashMap::new();
            let boundary = req.headers().get("Content-Type")
                .and_then(|ct| ct.to_str().ok())
                .and_then(|ct| ct.split("boundary=").nth(1))
                .ok_or_else(|| anyhow::anyhow!("No boundary found"));
            if let Err(err) = boundary {
                return Err(err.into())
            }
            let boundary = boundary.unwrap().to_string();
            let body = req.into_body().into_data_stream();
            let mut multipart = multer::Multipart::new(body, boundary);
            while let Some(field) = multipart.next_field().await? {
                let name = field.name().unwrap().to_string();
                if let Some(filename) = field.file_name() {
                    let filename = filename.to_string();
                    let content_type = field.content_type().map(|ct| ct.clone());
                    let bytes = field.bytes().await?;
                    let fil = FileItem {
                        filename,
                        size: bytes.len(),
                        bytes,
                        content_type,
                    };
                    files.entry(name).or_insert(vec![]).push(fil);
                } else {
                    let value = field.text().await?;
                    form.entry(name).or_insert(vec![]).push(value);
                }
            }
            Ok(Multipart { fields: form, files })
        })
    }
}

impl<S> FromRequestParts<S> for Method {
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<Self>
    where
        Self: Sized,
    {
        Box::pin(async move {
            Ok(req.method.clone())
        })
    }
}

impl<S> FromRequestParts<S> for Extensions {
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<Self>
    where
        Self: Sized,
    {
        Box::pin(async move {
            Ok(req.extensions.clone())
        })
    }
}

impl<S> FromRequestParts<S> for Uri {
    fn from_request_parts(req: &mut Parts, state: Arc<S>) -> FRPFut<Self>
    where
        Self: Sized,
    {
        Box::pin(async move {
            Ok(req.uri.clone())
        })
    }
}
use crate::ext::uploader::{FileField, UploadedFile};
use crate::handler::extractor::from_request::FromRequest;
use crate::handler::extractor::multipart::Multipart;
use crate::handler::handler::Req;
use crate::handler::into_response::IntoResponse;
use hyper::StatusCode;
use miko_core::Resp;
use miko_core::fast_builder::ResponseBuilder;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::Service;

/// 单文件上传服务（自动选择第一个文件字段进行处理）
#[derive(Clone)]
pub struct SingleUploader<H> {
    pub(crate) inner: Arc<H>,
}

/// 上传处理器：将一个上传字段处理为最终的 UploadedFile
pub trait UploaderProcesser {
    fn process(
        &self,
        file_field: FileField,
    ) -> impl Future<Output = Result<UploadedFile, anyhow::Error>> + Send + Sync + 'static;
}

impl<H> Service<Req> for SingleUploader<H>
where
    H: UploaderProcesser + Clone + Send + Sync + 'static,
{
    type Response = Resp;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: Req) -> Self::Future {
        let inner = self.inner.clone();
        Box::pin(async move {
            let Multipart(mut multipart) =
                Multipart::from_request(req, Arc::new(())).await.unwrap();
            let ffield;
            loop {
                let field = multipart.next_field().await;
                if let Err(e) = field {
                    return ResponseBuilder::internal_server_error(Some(e.to_string()));
                }
                if let Some(field) = field.unwrap() {
                    if field.file_name().is_some() {
                        ffield = Some(FileField {
                            original_filename: field.file_name().unwrap_or("").to_string(),
                            content_type: field.content_type().map(|s| s.clone()),
                            field,
                        });
                        break;
                    } else {
                        continue;
                    }
                } else {
                    return ResponseBuilder::internal_server_error(Some("No field".to_string()));
                }
            }
            let ffield = inner.process(ffield.unwrap()).await;
            match ffield {
                Ok(file) => {
                    ResponseBuilder::ok(format!("uploaded file {}", file.original_filename))
                }
                Err(e) => {
                    Ok((StatusCode::BAD_REQUEST, e.into_response()).into_response())
                }
            }
        })
    }
}

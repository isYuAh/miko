use crate::ext::uploader::{SingleUploader, UploaderProcesser};
use crate::handler::router::HttpSvc;
use miko_core::Req;
use std::sync::Arc;
use tower::util::BoxCloneService;

pub struct Uploader {}

impl Uploader {
    /// 创建单文件上传处理（仅处理遇到的第一个文件）
    pub fn single<T>(storage_provider: T) -> HttpSvc<Req>
    where
        T: UploaderProcesser + Clone + Send + Sync + 'static,
    {
        BoxCloneService::new(SingleUploader {
            inner: Arc::new(storage_provider),
        })
    }
}

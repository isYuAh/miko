use crate::ext::uploader::{SingleUploader, UploaderProcesser};
use crate::router::HttpSvc;
use miko_core::Req;
use std::sync::Arc;
use tower::util::BoxCloneService;

/// 上传功能入口，提供构建不同上传服务的便捷方法
pub struct Uploader {}

impl Uploader {
    /// 创建单文件上传处理（仅处理遇到的第一个文件，不限字段名）
    pub fn single<T>(storage_provider: T) -> HttpSvc<Req>
    where
        T: UploaderProcesser + Clone + Send + Sync + 'static,
    {
        BoxCloneService::new(SingleUploader {
            inner: Arc::new(storage_provider),
        })
    }
}

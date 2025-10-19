use crate::ext::uploader::{FileField, FileTransferConfig, UploadedFile, UploaderProcesser};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

/// 将上传文件保存到磁盘的存储器
#[derive(Clone)]
pub struct DiskStorage {
    pub root: PathBuf,
    pub config: DiskStorageConfig,
}
impl UploaderProcesser for DiskStorage {
    fn process(
        &self,
        file_field: FileField,
    ) -> impl Future<Output = Result<UploadedFile, anyhow::Error>> + Send + Sync + 'static {
        let root = self.root.clone();
        let config = self.config.clone();
        async move {
            let mut filename = file_field.original_filename.clone();
            if let Some(filename_mapper) = config.filename_mapper {
                filename = filename_mapper(&filename);
            }
            if let Some(allowed_extensions) = &config.allowed_extensions {
                let extension = filename.rsplit('.').next().unwrap_or("");
                if !allowed_extensions.contains(&extension.to_string()) {
                    return Err(anyhow::anyhow!("File extension not allowed"));
                }
            }
            if let Some(allowed_mime_types) = &config.allowed_mime_types {
                let mime_type = mime_guess::from_path(&filename).first_or_octet_stream();
                if !allowed_mime_types.contains(&mime_type.to_string()) {
                    return Err(anyhow::anyhow!("File mime type not allowed"));
                }
            }
            file_field
                .transfer_to(
                    root,
                    &filename,
                    FileTransferConfig {
                        max_size: config.max_size.clone(),
                    },
                )
                .await
        }
    }
}
impl DiskStorage {
    /// 创建一个磁盘存储器
    pub fn new(root: impl Into<PathBuf>, config: DiskStorageConfig) -> Self {
        Self {
            root: root.into(),
            config,
        }
    }
}

/// 磁盘存储配置：文件大小/扩展名/MIME 限制以及文件名映射
#[derive(Clone)]
pub struct DiskStorageConfig {
    pub max_size: Option<usize>,
    pub allowed_extensions: Option<Vec<String>>,
    pub allowed_mime_types: Option<Vec<String>>,
    pub filename_mapper: Option<Arc<dyn Fn(&str) -> String + Send + Sync + 'static>>,
}
impl Debug for DiskStorageConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskStorageConfig")
            .field("max_size", &self.max_size)
            .field("allowed_extensions", &self.allowed_extensions)
            .field("allowed_mime_types", &self.allowed_mime_types)
            .field("filename_mapper status", &self.filename_mapper.is_some())
            .finish()
    }
}
impl Default for DiskStorageConfig {
    fn default() -> Self {
        Self {
            max_size: None,
            allowed_extensions: None,
            allowed_mime_types: None,
            filename_mapper: None,
        }
    }
}
impl DiskStorageConfig {
    /// 限制最大文件尺寸（字节）
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.max_size = Some(max_size);
        self
    }
    /// 允许的扩展名白名单（不含点），如 ["png", "jpg"]
    pub fn allowed_extensions(mut self, allowed_extensions: Vec<String>) -> Self {
        self.allowed_extensions = Some(allowed_extensions);
        self
    }
    /// 允许的 MIME 类型白名单，如 ["image/png"]
    pub fn allowed_mime_types(mut self, allowed_mime_types: Vec<String>) -> Self {
        self.allowed_mime_types = Some(allowed_mime_types);
        self
    }
    /// 文件名映射，便于重命名（如追加时间戳/UUID）
    pub fn filename_mapper(
        mut self,
        filename_mapper: impl Fn(&str) -> String + Send + Sync + 'static,
    ) -> Self {
        self.filename_mapper = Some(Arc::new(filename_mapper));
        self
    }
}

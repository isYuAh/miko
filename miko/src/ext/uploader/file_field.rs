use bytes::Bytes;
use futures::{Stream, StreamExt};
use mime_guess::Mime;
use multer::{Error, Field};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct FileField {
    pub original_filename: String,
    pub content_type: Option<Mime>,
    pub field: Field<'static>,
}

#[derive(Debug)]
pub struct UploadedFile {
    pub original_filename: String,
    pub final_filename: String,
    pub size: usize,
    pub content_type: Option<Mime>,
}

impl Stream for FileField {
    type Item = Result<Bytes, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.field.poll_next_unpin(cx)
    }
}

impl FileField {
    pub fn from(field: Field<'static>) -> Self {
        let filename = field.file_name().map(|s| s.to_string()).unwrap_or_default();
        let content_type = field.content_type().map(|v| v.clone());
        Self {
            original_filename: filename,
            content_type,
            field,
        }
    }
}

impl FileField {
    pub async fn transfer_to(
        mut self,
        path: impl Into<PathBuf>,
        filename: &str,
        config: FileTransferConfig,
    ) -> Result<UploadedFile, anyhow::Error> {
        let path = path.into();
        tokio::fs::create_dir_all(&path).await?;
        let dest = path.join(filename);
        let mut dest_file = tokio::fs::File::create(&dest).await?;
        let mut size = 0;
        while let Some(chunk) = self.next().await {
            let chunk = chunk?;
            size += chunk.len();
            dest_file.write_all(&chunk).await?;
            if let Some(max_size) = config.max_size {
                if size > max_size {
                    dest_file.shutdown().await?;
                    tokio::fs::remove_file(&dest).await?;
                    return Err(anyhow::anyhow!("File size exceeded"));
                }
            }
        }
        Ok(UploadedFile {
            original_filename: self.original_filename,
            final_filename: filename.to_string(),
            size,
            content_type: self.content_type,
        })
    }
}

pub struct FileTransferConfig {
    pub max_size: Option<usize>,
}
impl Default for FileTransferConfig {
    fn default() -> Self {
        Self { max_size: None }
    }
}

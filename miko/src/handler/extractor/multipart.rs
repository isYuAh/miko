use crate::handler::extractor::from_request::{FRFut, FromRequest};
use crate::handler::handler::Req;
use bytes::Bytes;
use futures::TryStreamExt;
use http_body_util::BodyExt;
use hyper::HeaderMap;
use mime_guess::Mime;
use std::collections::HashMap;
use std::fs::Metadata;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

pub struct Multipart(pub multer::Multipart<'static>);
#[derive(Debug)]
pub struct MultipartResult {
    pub fields: HashMap<String, Vec<String>>,
    pub files: HashMap<String, Vec<MultipartFile>>,
}
#[derive(Debug)]
pub struct MultipartFile {
    pub filename: String,
    pub size: usize,
    pub content_type: Option<Mime>,
    pub linker: MultipartFileDiskLinker,
}
#[derive(Debug)]
pub struct MultipartFileDiskLinker {
    pub file: File,
    pub file_path: PathBuf,
    #[allow(dead_code)]
    temp_file: Arc<NamedTempFile>,
}

impl MultipartFileDiskLinker {
    pub async fn transfer_to(&self, path: impl Into<PathBuf>) -> Result<u64, std::io::Error> {
        Ok(tokio::fs::copy(self.file_path.clone(), path.into()).await?)
    }
    pub async fn read_to_string(&mut self) -> Result<String, std::io::Error> {
        let mut buf = String::new();
        self.file.read_to_string(&mut buf).await?;
        Ok(buf)
    }
    pub async fn read_and_drop_file(mut self) -> Result<Bytes, std::io::Error> {
        let mut buf = Vec::new();
        self.file.read_to_end(&mut buf).await?;
        Ok(Bytes::from(buf))
    }
    pub async fn metadata(&self) -> std::io::Result<Metadata> {
        self.file.metadata().await
    }
}

impl<S> FromRequest<S> for MultipartResult {
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let mut form = HashMap::new();
            let mut files = HashMap::new();
            let boundary = parse_boundary(req.headers());
            if let Err(err) = boundary {
                return Err(err.into());
            }
            let boundary = boundary.unwrap().to_string();
            let body = req.into_body().into_data_stream();
            let mut multipart = multer::Multipart::new(body, boundary);
            while let Some(field) = multipart.next_field().await? {
                let name = field.name().unwrap().to_string();
                if let Some(filename) = field.file_name() {
                    let filename = filename.to_string();
                    let content_type = field.content_type().map(|ct| ct.clone());
                    let temp_file = tempfile::NamedTempFile::new()?;
                    let file_path = temp_file.path().to_path_buf();
                    let mut async_file_writer = File::options()
                        .read(true)
                        .write(true)
                        .open(file_path.clone())
                        .await?;
                    let mut reader = StreamReader::new(
                        field
                            .into_stream()
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
                    );
                    tokio::io::copy(&mut reader, &mut async_file_writer).await?;
                    let fil = MultipartFile {
                        filename,
                        size: async_file_writer.metadata().await?.len() as usize,
                        content_type,
                        linker: MultipartFileDiskLinker {
                            file: async_file_writer,
                            file_path,
                            temp_file: Arc::new(temp_file),
                        },
                    };

                    files.entry(name).or_insert(vec![]).push(fil);
                } else {
                    let value = field.text().await?;
                    form.entry(name).or_insert(vec![]).push(value);
                }
            }
            Ok(MultipartResult {
                fields: form,
                files,
            })
        })
    }
}

impl<S> FromRequest<S> for Multipart {
    fn from_request(mut req: Req, _state: Arc<S>) -> FRFut<Self> {
        Box::pin(async move {
            let boundary = parse_boundary(req.headers());
            if let Err(err) = boundary {
                return Err(err.into());
            }
            let boundary = boundary.unwrap().to_string();
            let body = req.into_body().into_data_stream();
            let multipart = multer::Multipart::new(body, boundary);
            Ok(Multipart(multipart))
        })
    }
}

fn parse_boundary(headers: &HeaderMap) -> Result<String, anyhow::Error> {
    headers
        .get("Content-Type")
        .and_then(|ct| ct.to_str().ok())
        .and_then(|ct| ct.split("boundary=").nth(1))
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No boundary found"))
}

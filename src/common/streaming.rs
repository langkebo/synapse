use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::mpsc;

pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;
pub const MAX_UPLOAD_SIZE: usize = 100 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub chunk_size: usize,
    pub max_size: usize,
    pub buffer_size: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            max_size: MAX_UPLOAD_SIZE,
            buffer_size: 64 * 1024,
        }
    }
}

pub struct StreamReader {
    inner: BufReader<File>,
    chunk_size: usize,
    bytes_read: usize,
    max_size: usize,
}

impl StreamReader {
    pub async fn new(path: &std::path::Path, config: &StreamConfig) -> io::Result<Self> {
        let file = File::open(path).await?;
        Ok(Self {
            inner: BufReader::with_capacity(config.buffer_size, file),
            chunk_size: config.chunk_size,
            bytes_read: 0,
            max_size: config.max_size,
        })
    }

    pub async fn read_chunk(&mut self) -> io::Result<Option<Bytes>> {
        let mut buffer = vec![0u8; self.chunk_size];
        let n = self.inner.read(&mut buffer).await?;

        if n == 0 {
            return Ok(None);
        }

        self.bytes_read += n;
        if self.bytes_read > self.max_size {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Stream exceeded maximum size",
            ));
        }

        buffer.truncate(n);
        Ok(Some(Bytes::from(buffer)))
    }

    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }
}

pub struct StreamWriter {
    inner: BufWriter<File>,
    bytes_written: usize,
    max_size: usize,
}

impl StreamWriter {
    pub async fn new(path: &std::path::Path, config: &StreamConfig) -> io::Result<Self> {
        let file = File::create(path).await?;
        Ok(Self {
            inner: BufWriter::with_capacity(config.buffer_size, file),
            bytes_written: 0,
            max_size: config.max_size,
        })
    }

    pub async fn write_chunk(&mut self, chunk: &[u8]) -> io::Result<()> {
        if self.bytes_written + chunk.len() > self.max_size {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Stream exceeded maximum size",
            ));
        }

        self.inner.write_all(chunk).await?;
        self.bytes_written += chunk.len();
        Ok(())
    }

    pub async fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().await
    }

    pub fn bytes_written(&self) -> usize {
        self.bytes_written
    }
}

pub async fn copy_stream<S, W>(mut stream: S, mut writer: W, config: &StreamConfig) -> io::Result<usize>
where
    S: Stream<Item = Result<Bytes, io::Error>> + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut total_bytes = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;

        if total_bytes + chunk.len() > config.max_size {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Stream exceeded maximum size",
            ));
        }

        writer.write_all(&chunk).await?;
        total_bytes += chunk.len();
    }

    writer.flush().await?;
    Ok(total_bytes)
}

pub struct ChunkedUploader {
    config: StreamConfig,
    chunks: mpsc::Sender<UploadChunk>,
    total_bytes: usize,
    upload_id: String,
}

#[derive(Debug, Clone)]
pub struct UploadChunk {
    pub upload_id: String,
    pub chunk_index: usize,
    pub data: Bytes,
    pub is_last: bool,
}

#[derive(Debug, Clone)]
pub struct UploadProgress {
    pub upload_id: String,
    pub bytes_received: usize,
    pub chunks_received: usize,
    pub complete: bool,
}

impl ChunkedUploader {
    pub fn new(upload_id: String, config: StreamConfig) -> (Self, mpsc::Receiver<UploadChunk>) {
        let (chunks_tx, chunks_rx) = mpsc::channel(32);

        (
            Self {
                config,
                chunks: chunks_tx,
                total_bytes: 0,
                upload_id,
            },
            chunks_rx,
        )
    }

    pub async fn write(&mut self, data: &[u8]) -> io::Result<()> {
        if self.total_bytes + data.len() > self.config.max_size {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Upload exceeded maximum size",
            ));
        }

        self.total_bytes += data.len();
        Ok(())
    }

    pub async fn send_chunk(&self, chunk_index: usize, data: Bytes, is_last: bool) -> io::Result<()> {
        self.chunks
            .send(UploadChunk {
                upload_id: self.upload_id.clone(),
                chunk_index,
                data,
                is_last,
            })
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

pub struct ChunkedDownloader {
    config: StreamConfig,
    chunk_index: usize,
    total_bytes: usize,
}

impl ChunkedDownloader {
    pub fn new(config: StreamConfig) -> Self {
        Self {
            config,
            chunk_index: 0,
            total_bytes: 0,
        }
    }

    pub async fn read_chunk<R>(&mut self, mut reader: R) -> io::Result<Option<Bytes>>
    where
        R: AsyncRead + Unpin,
    {
        let mut buffer = vec![0u8; self.config.chunk_size];
        let n = reader.read(&mut buffer).await?;

        if n == 0 {
            return Ok(None);
        }

        self.chunk_index += 1;
        self.total_bytes += n;
        buffer.truncate(n);
        Ok(Some(Bytes::from(buffer)))
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

pub struct MemoryStream {
    buffer: Vec<u8>,
    position: usize,
}

impl MemoryStream {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            position: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            position: 0,
        }
    }

    pub fn from_bytes(bytes: Bytes) -> Self {
        Self {
            buffer: bytes.to_vec(),
            position: 0,
        }
    }

    pub async fn write(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub async fn read(&mut self, size: usize) -> Option<Bytes> {
        if self.position >= self.buffer.len() {
            return None;
        }

        let end = (self.position + size).min(self.buffer.len());
        let slice = self.buffer[self.position..end].to_vec();
        self.position = end;
        Some(Bytes::from(slice))
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn into_bytes(self) -> Bytes {
        Bytes::from(self.buffer)
    }
}

impl Default for MemoryStream {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RateLimitedStream<S> {
    inner: S,
    bytes_per_second: usize,
    bytes_this_second: usize,
    last_check: std::time::Instant,
}

impl<S> RateLimitedStream<S> {
    pub fn new(inner: S, bytes_per_second: usize) -> Self {
        Self {
            inner,
            bytes_per_second,
            bytes_this_second: 0,
            last_check: std::time::Instant::now(),
        }
    }
}

impl<S> Stream for RateLimitedStream<S>
where
    S: Stream<Item = Result<Bytes, io::Error>> + Unpin,
{
    type Item = Result<Bytes, io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_check);

        if elapsed.as_secs() >= 1 {
            self.bytes_this_second = 0;
            self.last_check = now;
        }

        if self.bytes_this_second >= self.bytes_per_second {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.bytes_this_second += chunk.len();
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_stream_writer_and_reader() {
        let config = StreamConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut writer = StreamWriter::new(path, &config).await.unwrap();
        writer.write_chunk(b"Hello, ").await.unwrap();
        writer.write_chunk(b"World!").await.unwrap();
        writer.flush().await.unwrap();
        assert_eq!(writer.bytes_written(), 13);

        let mut reader = StreamReader::new(path, &config).await.unwrap();
        let mut total = 0;
        while let Some(chunk) = reader.read_chunk().await.unwrap() {
            total += chunk.len();
        }
        assert_eq!(total, 13);
    }

    #[tokio::test]
    async fn test_memory_stream() {
        let mut stream = MemoryStream::with_capacity(100);
        stream.write(b"Hello").await;
        stream.write(b" World").await;

        assert_eq!(stream.len(), 11);

        let chunk = stream.read(5).await.unwrap();
        assert_eq!(&chunk[..], b"Hello");

        let chunk = stream.read(100).await.unwrap();
        assert_eq!(&chunk[..], b" World");
    }

    #[tokio::test]
    async fn test_chunked_uploader() {
        let config = StreamConfig::default();
        let (mut uploader, mut receiver) = ChunkedUploader::new("test-upload".to_string(), config);

        uploader.write(b"test data").await.unwrap();
        assert_eq!(uploader.total_bytes(), 9);

        uploader
            .send_chunk(0, Bytes::from("test data"), true)
            .await
            .unwrap();

        let chunk = receiver.recv().await.unwrap();
        assert_eq!(chunk.upload_id, "test-upload");
        assert_eq!(chunk.chunk_index, 0);
        assert!(chunk.is_last);
    }

    #[tokio::test]
    async fn test_max_size_limit() {
        let config = StreamConfig {
            chunk_size: 10,
            max_size: 20,
            buffer_size: 10,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut writer = StreamWriter::new(path, &config).await.unwrap();
        writer.write_chunk(b"1234567890").await.unwrap();
        writer.write_chunk(b"1234567890").await.unwrap();

        let result = writer.write_chunk(b"1").await;
        assert!(result.is_err());
    }
}

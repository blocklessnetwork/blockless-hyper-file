use std::{
    future::Future, 
    path::{Path, PathBuf}, 
    pin::Pin, 
    io::{Result, Error, ErrorKind},
    task::{Context, Poll}, 
    mem::MaybeUninit, 
    io::SeekFrom,
    cmp::min, 
    fs::{OpenOptions, Permissions}, 
    time::SystemTime
};

use hyper::body::Bytes;
use tokio::{
    io::{AsyncRead, ReadBuf, AsyncSeek}, 
    fs::File, 
    task::JoinHandle
};

const READ_BUF_SIZE: usize = 10240;

/// file with the meta use for body stream.
#[derive(Debug)]
pub struct FileWithMeta {
    pub size: u64,
    pub file: File,
    pub is_dir: bool,
    pub modified: Option<SystemTime>,
    pub permisions: Permissions,
}

/// The file reader which read the bytes from file to fill the body.
pub trait FileReader: AsyncSeek + Unpin + Send + 'static {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, readn: u64) -> Poll<Result<Bytes>>;
}

pub trait FileReaderOpener: Send + Sync + 'static {
    type Output: Into<FileWithMeta>;

    type Future: Future<Output = Result<Self::Output>> + Unpin + 'static;

    fn open<T: AsRef<Path>>(&self, path: T) -> Self::Future;
}

/// The file reader which read the bytes from file to fill the body.
/// Using th tokio file in tokio async runtime.
pub struct TokioFileReader {
    file: tokio::fs::File,
    buf: Box<[MaybeUninit<u8>; READ_BUF_SIZE]>,
}

impl TokioFileReader {
    fn new(file: File) -> Self {
        Self {
            file,
            buf: Box::new([MaybeUninit::uninit(); READ_BUF_SIZE]),
        }
    }
}

impl FileReader for TokioFileReader {

    /// read bytes from file to fill the http body.
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, readn: u64) -> Poll<Result<Bytes>> {
        let Self {
            ref mut file,
            ref mut buf,
        } = *self;
        let buf_len = min(readn as usize, buf.len());
        let mut buf = ReadBuf::uninit(&mut buf[..buf_len]);
        match Pin::new(file).poll_read(cx, &mut buf) {
            Poll::Ready(Ok(())) => {
                let bs = buf.filled();
                if bs.len() == 0 {
                    Poll::Ready(Ok(Bytes::new()))
                } else {
                    let bs = Bytes::copy_from_slice(bs);
                    Poll::Ready(Ok(bs))
                }
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncSeek for TokioFileReader {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> Result<()> {
        Pin::new(&mut self.get_mut().file).start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<u64>> {
        Pin::new(&mut self.get_mut().file).poll_complete(cx)
    }
}

impl Into<TokioFileReader> for FileWithMeta {
    fn into(self) -> TokioFileReader {
        TokioFileReader::new(self.file)
    }
}

/// The future get the file and meta info 
pub struct FileWithMetaFuture {
    inner: JoinHandle<Result<FileWithMeta>>
}

impl FileWithMetaFuture {
    fn new(path: PathBuf) -> Self {
        let inner = tokio::task::spawn_blocking(move || -> Result<FileWithMeta> {
            let file = OpenOptions::new()
                .read(true)
                .open(path)?;
            let meta = file.metadata()?;
            let file = tokio::fs::File::from_std(file);
            Ok(FileWithMeta {
                file,
                size: meta.len(),
                is_dir: meta.is_dir(),
                modified: meta.modified().ok(),
                permisions: meta.permissions(),
            })
        });
        Self { inner }
    }
}

impl Future for FileWithMetaFuture {
    type Output = Result<FileWithMeta>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // the result is Result<Result<FileWithMeta>>
        // Poll::Ready(Ok(r)) => Poll::Ready(r) mean return the Poll::Ready(Ok) or Poll::Ready(Err), flatten
        let p = Pin::new(&mut self.inner).poll(cx);
        match p {
            Poll::Ready(Ok(r)) => Poll::Ready(r),
            Poll::Ready(Err(_)) => {
                //only Joinhandle error.
                Poll::Ready(Err(Error::new(ErrorKind::Other, "error execute in background.")))
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct TokioFileReaderOpener {
    root: PathBuf,
}

impl TokioFileReaderOpener {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into()
        }
    }
}


impl FileReaderOpener for TokioFileReaderOpener {
    type Output = FileWithMeta;

    type Future = FileWithMetaFuture;

    fn open<T: AsRef<Path>>(&self, path: T) -> Self::Future {
        let mut full_path = self.root.clone();
        full_path.extend(path.as_ref());
        FileWithMetaFuture::new(full_path)
    }
}


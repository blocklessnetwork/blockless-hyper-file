use std::task::{Context, Poll};
use std::{path::PathBuf, pin::Pin};
use std::io::{Result, ErrorKind};
use std::future::Future;
use hyper::{Request, Method};

use crate::file::{
    TokioFileReaderOpener, 
    FileReaderOpener, FileWithMeta
};
#[derive(Debug)]
pub enum Resolved {
    MethodNotMatched,
    NotFound,
    PermissionDenied,
    IsDirectory,
    Found(FileWithMeta),
}

pub(crate) struct RequestResolve<'a, B, T = TokioFileReaderOpener> {
    opener: T,
    request: &'a Request<B>,
}

impl<'a, B, T: FileReaderOpener> RequestResolve<'a, B, T> {
    fn new_with_opener(t: T, r: &'a Request<B>) -> Self {
        Self { opener: t, request: r }
    }
}

impl<'a, B> RequestResolve<'a, B, TokioFileReaderOpener> {
    pub fn new(path: impl Into<PathBuf>, r: &'a Request<B>) -> Self {
        let opener = TokioFileReaderOpener::new(path);
        Self::new_with_opener(opener, r)
    }

    pub async fn resolve(&self) -> Result<Resolved> {
        let Self {
            ref opener,
            ref request,
        } = *self;
        match *request.method() {
            Method::GET| Method::HEAD => {},
            _ => return Ok(Resolved::MethodNotMatched),
        }
        let path = request.uri().path();
        let file_with_meta = match opener.open(path).await {
            Ok(r) => r,
            Err(e) =>  {
                let rs = match e.kind() {
                    ErrorKind::NotFound => Ok(Resolved::NotFound),
                    ErrorKind::PermissionDenied => Ok(Resolved::PermissionDenied),
                    e @ _ => Err(e.into()),
                };
                return rs;
            },
        };
        if file_with_meta.is_dir {
            return Ok(Resolved::IsDirectory);
        }
        return Ok(Resolved::Found(file_with_meta));
    }
}

impl<'a, B, T: FileReaderOpener<Output = FileWithMeta>> Future for RequestResolve<'a, B, T> {
    type Output = Result<Resolved>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            ref opener,
            ref request,
        } = *self;
        match *request.method() {
            Method::GET| Method::HEAD => {},
            _ => return Poll::Ready(Ok(Resolved::MethodNotMatched)),
        }
        let path = request.uri().path();
        let mut fut = opener.open(path);
        let p = Pin::new(&mut fut).poll(cx);
        let file_with_meta = match  p {
            Poll::Ready(Ok(r)) => r,
            Poll::Ready(Err(e)) =>  {
                let rs = match e.kind() {
                    
                    ErrorKind::NotFound => Ok(Resolved::NotFound),
                    ErrorKind::PermissionDenied => Ok(Resolved::PermissionDenied),
                    e @ _ => Err(e.into()),
                };
                
                return Poll::Ready(rs);
            },
            Poll::Pending => return Poll::Pending,
        };
        if file_with_meta.is_dir {
            return Poll::Ready(Ok(Resolved::IsDirectory));
        }
        return Poll::Ready(Ok(Resolved::Found(file_with_meta)));
    }
}
use std::task::{Context, Poll};
use std::{path::PathBuf, pin::Pin};
use std::io::{Result, ErrorKind};
use std::future::Future;
use hyper::{Request, Method};

use crate::file::{
    TokioFileReaderOpener, 
    FileReaderOpener, FileWithMeta, FileWithMetaFuture
};
#[derive(Debug)]
pub enum Resolved {
    NotFound,
    IsDirectory,
    MethodNotMatched,
    PermissionDenied,
    Found(FileWithMeta),
}

pub(crate) struct RequestResolve {
    opener_future: FileWithMetaFuture,
    is_method_match: bool,
}

fn decode_percents(string: &str) -> String {
    percent_encoding::percent_decode_str(string)
        .decode_utf8_lossy()
        .into_owned()
}

impl RequestResolve {
    pub fn resolve<B>(path: impl Into<PathBuf>, r: &Request<B>) -> Self {
        let opener = TokioFileReaderOpener::new(path);
        let mut uri_path = r.uri().path();
        if uri_path.starts_with("/") {
            uri_path = &uri_path[1..];
        }
        let opener_future = opener.open(decode_percents(uri_path));
        let is_method_match = match *r.method() {
            Method::GET| Method::HEAD => true,
            _ => false,
        };
        RequestResolve {
            opener_future,
            is_method_match,
        }
    }
}

impl Future for RequestResolve {
    type Output = Result<Resolved>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            ref mut opener_future,
            is_method_match,
        } = *self;
        if !is_method_match {
            return Poll::Ready(Ok(Resolved::MethodNotMatched));
        }
        let file_with_meta = match Pin::new(opener_future).poll(cx) {
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
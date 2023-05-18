use std::task::{Context, Poll};
use std::{path::PathBuf, pin::Pin};
use std::io::{Result, ErrorKind};
use std::future::Future;
use hyper::{Request, Method};

use crate::file::{
    TokioFileReaderOpener, 
    FileReaderOpener, FileWithMeta
};

pub enum Resolved {
    MethodNotMatched,
    NotFound,
    PermissionDenied,
    IsDirectory,
    Found,
}

struct RequestResolve {
    opener: TokioFileReaderOpener,
}

impl RequestResolve {
    fn new(path: impl Into<PathBuf>) -> Self {
        let opener = TokioFileReaderOpener {root: path.into()};
        Self::new_with_opener(opener)
    }

    fn new_with_opener(o: TokioFileReaderOpener) -> Self {
        Self {
            opener: o
        }
    }

    pub fn resolve<'a, 'b, B>(&'a mut self, request: &'b Request<B>) -> ResolveFuture<'a, 'b, B> {
        ResolveFuture {
            inner: self,
            request,
        }
    }
}


struct ResolveFuture<'a, 'b, B> {
    inner: &'a mut RequestResolve,
    request:&'b Request<B>
}

impl<'a, 'b, B> Future for ResolveFuture<'a, 'b, B> {
    type Output = Result<Resolved>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            inner: ref mut req_resolve,
            ref mut request,
        } = *self;
        match *request.method() {
            Method::GET| Method::HEAD => {},
            _ => return Poll::Ready(Ok(Resolved::MethodNotMatched)),
        }
        let path = request.uri().path();
        let mut fut = req_resolve.opener.open(path);
        let file_with_meta = match Pin::new(&mut fut).poll(cx) {
            Poll::Ready(Ok(r)) => r,
            Poll::Ready(Err(e)) =>  {
                let rs = match e.kind() {
                    ErrorKind::NotFound => Ok(Resolved::MethodNotMatched),
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
        return Poll::Ready(Ok(Resolved::Found));
    }
}
use futures_util::Stream;
use hyper::body::Bytes;
use std::{io::Result, task::Poll, pin::Pin};

use crate::file::{TokioFileReader, FileReader};


pub struct FileBytesStream<T = TokioFileReader> {
    pub reader: T,
    pub(crate) remaining: u64,
}

impl<T>  FileBytesStream<T>  {
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            remaining: u64::MAX
        }
    }

    pub fn new_with_limited(reader: T, limited: u64) -> Self {
        Self {
            reader,
            remaining: limited
        }
    }
}

impl<T: FileReader> Stream for FileBytesStream<T> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let Self {
            ref mut reader,
            ref mut remaining,
        } = *self;
        match Pin::new(reader).poll_read(cx, *remaining) {
            Poll::Ready(Ok(b)) => {
                *remaining -= b.len() as u64;
                if b.is_empty() {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(b)))
                }
            },
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}



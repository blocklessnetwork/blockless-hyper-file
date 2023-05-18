use hyper::body::Bytes;
use range_bytes_stream::MultiRangeBytesStream;
use std::{
    io::Error, 
    task::{Poll, Context}, 
    pin::Pin
};
use futures_util::Stream;

mod bytes_stream;
mod range_bytes_stream;
mod chunked_bytes_stream;

pub enum Body {
    MultiRangeBytesStream(MultiRangeBytesStream)
}

impl hyper::body::HttpBody for Body {
    type Data = Bytes;

    type Error = Error;

    fn poll_data(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        match *self {
            Body::MultiRangeBytesStream(ref mut r) => Pin::new(r).poll_next(cx),
        }
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<hyper::HeaderMap>, Self::Error>> {
        Poll::Ready(Ok(None))
    }
}
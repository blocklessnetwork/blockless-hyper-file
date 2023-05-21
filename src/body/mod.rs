use hyper::body::Bytes;

use futures_util::Stream;
use std::{
    io::Error,
    pin::Pin,
    task::{Context, Poll},
};

pub use bytes_stream::FileBytesStream;
pub use range_bytes_stream::MultiRangeBytesStream;
pub use range_bytes_stream::RangeBytesStream;

mod bytes_stream;
mod chunked_bytes_stream;
mod range_bytes_stream;

pub enum Body {
    Empty,
    Full(FileBytesStream),
    RangeBytesStream(RangeBytesStream),
    MultiRangeBytesStream(MultiRangeBytesStream),
}

impl hyper::body::HttpBody for Body {
    type Data = Bytes;

    type Error = Error;

    fn poll_data(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        match *self {
            Body::MultiRangeBytesStream(ref mut mr) => Pin::new(mr).poll_next(cx),
            Body::RangeBytesStream(ref mut r) => Pin::new(r).poll_next(cx),
            Body::Full(ref mut r) => Pin::new(r).poll_next(cx),
            Body::Empty => Poll::Ready(None),
        }
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Option<hyper::HeaderMap>, Self::Error>> {
        Poll::Ready(Ok(None))
    }
}

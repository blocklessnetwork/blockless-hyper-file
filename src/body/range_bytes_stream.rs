use std::pin::Pin;
use std::task::{Context, Poll};

use std::io::{Result, SeekFrom};
use futures_util::Stream;
use hyper::body::Bytes;

use crate::range::HttpRange;

use super::bytes_stream::FileBytesStream;

#[derive(Debug, Clone, Copy)]
enum RangeState {
    Inital,
    Seeking,
    Reading,
}

struct RangeBytesStream {
    state: RangeState,
    range: HttpRange,
    stream: FileBytesStream,
}

impl Stream for RangeBytesStream {
    type Item = Result<Bytes>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Self {
            ref mut stream,
            ref mut state,
            ref mut range,
        } = *self;
        if let RangeState::Inital = *state {
            let seek_position = SeekFrom::Start(range.start);
            if let Err(e) = Pin::new(&mut stream.reader).start_seek(seek_position) {
                return Poll::Ready(Some(Err(e)));
            }
            *state = RangeState::Reading;
        }
        if let RangeState::Seeking = *state {
            match Pin::new(stream).poll_complete(cx) {
                Poll::Ready(Ok(_)) => {
                    *state = RangeState::Reading;
                },
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Some(Err(e.into())))
                },
                Poll::Pending => return Poll::Pending,
            };
        }
        Pin::new(stream).poll_next(cx)
    }
   
}

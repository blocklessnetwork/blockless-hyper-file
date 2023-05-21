use std::fmt::Write;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use hyper::body::Bytes;
use std::io::{Result, SeekFrom};
use std::vec;
use tokio::io::AsyncSeek;

use crate::file::TokioFileReader;
use crate::range::HttpRange;

use super::bytes_stream::FileBytesStream;

#[derive(Debug, Clone, Copy)]
enum RangeState {
    Inital,
    Seeking,
    Reading,
}

pub struct RangeBytesStream {
    state: RangeState,
    start_pos: u64,
    stream: FileBytesStream,
}

impl RangeBytesStream {
    pub fn new_with_range(reader: TokioFileReader, range: &HttpRange) -> RangeBytesStream {
        Self {
            stream: FileBytesStream::new_with_limited(reader, range.length),
            start_pos: range.start,
            state: RangeState::Inital,
        }
    }
    pub fn new(reader: TokioFileReader) -> RangeBytesStream {
        RangeBytesStream {
            stream: FileBytesStream::new_with_limited(reader, 0),
            state: RangeState::Inital,
            start_pos: 0,
        }
    }
}

impl Stream for RangeBytesStream {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Self {
            ref mut stream,
            ref mut state,
            start_pos,
        } = *self;
        if let RangeState::Inital = *state {
            let seek_position = SeekFrom::Start(start_pos);
            *state = RangeState::Seeking;
            if let Err(e) = Pin::new(&mut stream.reader).start_seek(seek_position) {
                return Poll::Ready(Some(Err(e)));
            }
        }
        if let RangeState::Seeking = *state {
            match Pin::new(&mut stream.reader).poll_complete(cx) {
                Poll::Ready(Ok(_)) => {
                    *state = RangeState::Reading;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
                Poll::Pending => return Poll::Pending,
            };
        }
        Pin::new(stream).poll_next(cx)
    }
}

pub struct MultiRangeBytesStream {
    ranges: vec::IntoIter<HttpRange>,
    range_stream: RangeBytesStream,
    is_first_boundary: bool,
    completed: bool,
    boundary: String,
    content_type: String,
    file_size: u64,
}

impl MultiRangeBytesStream {
    pub fn new(
        reader: TokioFileReader,
        ranges: Vec<HttpRange>,
        boundary: String,
        file_size: u64,
    ) -> Self {
        Self {
            ranges: ranges.into_iter(),
            is_first_boundary: true,
            completed: false,
            boundary,
            range_stream: RangeBytesStream::new(reader),
            file_size,
            content_type: String::new(),
        }
    }

    pub fn set_content_type(&mut self, content_type: String) {
        self.content_type = content_type;
    }

    /// compute the body length for set the Content-Length
    pub fn compute_body_len(&self) -> u64 {
        let Self {
            ref ranges,
            ref boundary,
            ref content_type,
            file_size,
            ..
        } = *self;
        let mut is_first = true;
        let total: u64 = ranges
            .as_slice()
            .iter()
            .map(|range| {
                let header =
                    Self::render_header(boundary, is_first, file_size, range, content_type);
                is_first = false;
                header.len() as u64 + range.length
            })
            .sum();
        let header_end = Self::render_header_end(boundary);
        total + header_end.len() as u64
    }

    /// render the header of multi-part.
    fn render_header(
        boundary: &str,
        is_first: bool,
        file_size: u64,
        range: &HttpRange,
        content_type: &str,
    ) -> String {
        let mut buf = String::with_capacity(128);
        if !is_first {
            //new line split the content
            buf.push_str("\r\n");
        }
        write!(
            &mut buf,
            "--{boundary}\r\nContent-Range: bytes {}-{}/{file_size}\r\n",
            range.start, range.length,
        )
        .expect("buf write error");

        if !content_type.is_empty() {
            write!(&mut buf, "Content-Type: {content_type}\r\n").expect("buffer write failed");
        }
        buf.push_str("\r\n");
        buf
    }

    /// render the header of multi-part for end of body.
    fn render_header_end(boundary: &str) -> String {
        format!("\r\n--{boundary}--\r\n")
    }
}

impl Stream for MultiRangeBytesStream {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Self {
            ref mut ranges,
            ref mut range_stream,
            ref mut is_first_boundary,
            ref mut completed,
            ref boundary,
            ref content_type,
            file_size,
        } = *self;
        if *completed {
            return Poll::Ready(None);
        }
        if range_stream.stream.remaining == 0 {
            let range = match ranges.next() {
                Some(range) => range,
                None => {
                    *completed = true;
                    let header_end = Self::render_header_end(boundary);
                    return Poll::Ready(Some(Ok(header_end.into())));
                }
            };
            let is_first = *is_first_boundary;
            range_stream.state = RangeState::Inital;
            range_stream.start_pos = range.start;
            range_stream.stream.remaining = range.length;
            *is_first_boundary = false;
            let header = Self::render_header(boundary, is_first, file_size, &range, content_type);
            return Poll::Ready(Some(Ok(header.into())));
        }
        Pin::new(range_stream).poll_next(cx)
    }
}

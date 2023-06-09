use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hyper::{header, http::Result, HeaderMap, Method, Request, Response, StatusCode};

use crate::{
    body::{Body, FileBytesStream, MultiRangeBytesStream, RangeBytesStream},
    file::FileWithMeta,
    range::HttpRange,
};

const VALID_MTIME: Duration = Duration::from_secs(2);
const BOUNDARY_LEN: u64 = 35;
const BOUNDARY_CHRS: &[u8] = b"abcdefghghijkmlnopqrstuvwxyz0123456789ABCDEFGHGHIJKMLNOPQRSTUVWXYZ";

#[derive(Default, Debug, Clone)]
pub struct ResponseBuilder {
    //range from request.
    range: Option<String>,
    // `If-Modified-Since` request header.
    if_modified_since: Option<SystemTime>,
    // `If-Range` request header.
    if_range: Option<String>,
    is_head_method: bool,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    fn random_boundary() -> String {
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
        let mut rnd = duration.map(|d| d.as_secs()).unwrap();
        let mut boundary_buf = vec![0u8; (BOUNDARY_LEN + 10) as _];
        for iter in boundary_buf.iter_mut().skip(10) {
            let idx = (rnd % BOUNDARY_LEN) as usize;
            *iter = BOUNDARY_CHRS[idx];
            rnd += 1;
        }
        boundary_buf[0..10].copy_from_slice(b"blockless:");
        String::from_utf8(boundary_buf).unwrap()
    }

    pub fn range_header(&mut self, value: Option<&header::HeaderValue>) -> &mut Self {
        self.range = value.and_then(|v| v.to_str().ok()).map(String::from);
        self
    }

    pub fn request<B>(&mut self, req: &Request<B>) -> &mut Self {
        self.request_headers(req.headers());
        self.is_head_method(req.method());
        self
    }

    pub fn request_headers(&mut self, headers: &HeaderMap) -> &mut Self {
        self.range_header(headers.get(header::RANGE));
        self.if_modified_since_header(headers.get(header::IF_MODIFIED_SINCE));
        self.if_range_header(headers.get(header::IF_RANGE));
        self
    }

    fn if_modified_since_header(&mut self, value: Option<&header::HeaderValue>) -> &mut Self {
        self.if_modified_since = value
            .and_then(|v| v.to_str().ok())
            .and_then(|v| httpdate::parse_http_date(v).ok());
        self
    }

    fn if_range_header(&mut self, value: Option<&header::HeaderValue>) -> &mut Self {
        self.if_range = value.and_then(|v| v.to_str().ok()).map(String::from);
        self
    }

    fn is_head_method(&mut self, method: &Method) -> &mut Self {
        self.is_head_method = method == Method::HEAD;
        self
    }

    fn content_range_header(range: &HttpRange, file_size: u64) -> String {
        format!("bytes {}-{}/{}", range.start, range.length, file_size)
    }

    pub fn build(&self, file: FileWithMeta) -> Result<Response<Body>> {
        let file_size = file.size;
        let mut resp_builder = Response::builder();
        let modified = file.modified.filter(|m| {
            m.duration_since(UNIX_EPOCH)
                .ok()
                .filter(|d| d >= &VALID_MTIME)
                .is_some()
        });
        if let Some(modified) = modified {
            if let Ok(unix_time) = modified.duration_since(UNIX_EPOCH) {
                let ims_unix_time = self.if_modified_since.map(|t| t.duration_since(UNIX_EPOCH));
                if let Some(Ok(ims_unix_time)) = ims_unix_time {
                    if unix_time.as_secs() <= ims_unix_time.as_secs() {
                        return resp_builder
                            .status(StatusCode::NOT_MODIFIED)
                            .body(Body::Empty);
                    }
                }
            }
            let last_modified = httpdate::fmt_http_date(modified);
            resp_builder = resp_builder.header(header::LAST_MODIFIED, last_modified);
            resp_builder = resp_builder.header(header::ACCEPT_RANGES, "bytes");
        }
        let ranges = self.range.as_ref().map(|s| HttpRange::parse(s, file_size));
        if self.is_head_method {
            resp_builder = resp_builder.header(header::CONTENT_LENGTH, format!("{}", file_size));
            return resp_builder.status(StatusCode::OK).body(Body::Empty);
        }
        if let Some(ranges) = ranges {
            let ranges = match ranges {
                Ok(r) => r,
                Err(_) => {
                    return resp_builder
                        .status(StatusCode::RANGE_NOT_SATISFIABLE)
                        .body(Body::Empty)
                }
            };
            let ranges_len = ranges.len();
            
            #[allow(clippy::comparison_chain)]
            if ranges_len == 1 {
                let range = &ranges[0];
                let content_range_header = Self::content_range_header(range, file.size);
                resp_builder = resp_builder
                    .header(header::CONTENT_RANGE, content_range_header)
                    .header(header::CONTENT_LENGTH, file_size);
                let stream = RangeBytesStream::new_with_range(file.into(), range);
                return resp_builder
                    .status(StatusCode::PARTIAL_CONTENT)
                    .body(Body::RangeBytesStream(stream));
            } else if ranges_len > 1 {
                let boundary = Self::random_boundary();
                let content_type = format!("multipart/byteranges; boundary={}", &boundary);
                let stream = MultiRangeBytesStream::new(file.into(), ranges, boundary, file_size);
                resp_builder = resp_builder
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::CONTENT_LENGTH, stream.compute_body_len());
                return resp_builder
                    .status(StatusCode::PARTIAL_CONTENT)
                    .body(Body::MultiRangeBytesStream(stream));
            }
        }
        resp_builder = resp_builder.header(header::CONTENT_LENGTH, file_size);
        let stream = FileBytesStream::new_with_limited(file.into(), file_size);
        resp_builder.status(StatusCode::OK).body(Body::Full(stream))
    }
}

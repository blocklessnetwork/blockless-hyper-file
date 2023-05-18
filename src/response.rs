use std::time::SystemTime;

use hyper::{header, HeaderMap, http, Response};
use httpdate;

use crate::body::Body;

#[derive(Default, Debug, Clone)]
pub struct ResponseBuilder {
    //range from request.
    pub range: Option<String>,
    /// `If-Modified-Since` request header.
    pub if_modified_since: Option<SystemTime>,
}

impl ResponseBuilder {

    pub fn range_header(&mut self, value: Option<&header::HeaderValue>) -> &mut Self {
        self.range = value
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        self
    }

    pub fn request_headers(&mut self, headers: &HeaderMap) -> &mut Self {
        self.range_header(headers.get(header::RANGE));
        self.if_modified_since_header(headers.get(header::IF_MODIFIED_SINCE));
        self
    }
    
    pub fn if_modified_since_header(&mut self, value: Option<&header::HeaderValue>) -> &mut Self {
        self.if_modified_since = value
            .and_then(|v| v.to_str().ok())
            .and_then(|v| httpdate::parse_http_date(v).ok());
        self
    }

    pub fn build() -> http::Result<Response<Body>> {
        todo!()
    }
}
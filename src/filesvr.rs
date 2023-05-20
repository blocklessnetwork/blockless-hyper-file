use std::{
    io::{Error, Result},
    pin::Pin, 
    task::{Poll, Context}
};

use hyper::{
    service::Service, 
    Request, 
    Response, 
    StatusCode,
};

use std::future::Future;

use crate::{
    request_resolve::{
        RequestResolve, 
        Resolved
    }, 
    resp_builder::ResponseBuilder, 
    body::Body
};

pub struct FileSvr {
    local_root: String
}

impl Clone for FileSvr {
    fn clone(&self) -> Self {
        Self { 
            local_root: self.local_root.clone()
        }
    }
}

impl<B> Service<Request<B>> for FileSvr 
where
    B: Sync + Send + 'static
{
    type Response = Response<Body>;

    type Error = Error;

    type Future = ResponseFuture<B>;

    fn call(&mut self, req: Request<B>) -> Self::Future {
        ResponseFuture::new(self.local_root.clone(), req)
    }

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

}

pub struct ResponseFuture<B> {
    request: Request<B>,
    local_root: String,
}

impl<B> ResponseFuture<B> {
    fn new(local_root: String, request: Request<B>) -> Self {
        Self { 
            request, 
            local_root 
        }
    }
}

impl<B> Future for ResponseFuture<B> {
    type Output = Result<Response<Body>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self{
            request: ref req,
            ref local_root
        } = *self;
        let mut req_resolve = RequestResolve::new(local_root, &req);
        let resolved = match Pin::new(&mut req_resolve).poll(cx) {
            Poll::Ready(Ok(r)) => r,
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        };
        let resp = match resolved {
            Resolved::IsDirectory => Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::Empty),
            Resolved::MethodNotMatched => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::Empty),
            Resolved::NotFound => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::Empty),
            Resolved::PermissionDenied => Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(Body::Empty),
            Resolved::Found(f) => ResponseBuilder::new().build(f),
        }.unwrap();
        Poll::Ready(Ok(resp))
    }
}
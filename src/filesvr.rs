use std::{
    io::{Error, Result, ErrorKind},
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
    body::Body, file::{TokioFileReaderOpener, FileReaderOpener}
};

#[derive(Clone)]
pub struct FileSvr {
    local_root: String
}

impl FileSvr {
    pub fn new(root: impl Into<String>) -> FileSvr {
        Self {
            local_root: root.into()
        }
    }

    pub async fn serv<B>(self, request: Request<B>) -> Result<Response<Body>> {
        let resolved = RequestResolve::new(self.local_root.clone(), &request).resolve().await?;
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
        };
        let resp = match resp {
            Ok(resp) => resp,
            Err(e) => {
                let e = Error::new(ErrorKind::Other, e);
                return Err(e);
            },
        };
        Ok(resp)
    }
}

impl<B> Service<Request<B>> for FileSvr 
where
    B: Sync + Send + 'static
{
    type Response = Response<Body>;

    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>>>+Send>>;

    fn call(&mut self, req: Request<B>) -> Self::Future {
        Box::pin(self.clone().serv(req))
    }

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

}

pub struct RequestFuture<B> {
    request: Request<B>,
    local_root: String,
}

impl<B> RequestFuture<B> {
    fn new(local_root: String, request: Request<B>) -> Self {
        Self { 
            request, 
            local_root 
        }
    }
}

impl<B> Future for RequestFuture<B> {
    type Output = Result<Response<Body>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self{
            request: ref req,
            ref local_root
        } = *self;
        let mut req_resolve = RequestResolve::new(local_root, &req);
        let resolved = match Pin::new(&mut req_resolve).poll(cx) {
            Poll::Ready(Ok(r)) => r,
            Poll::Ready(Err(e)) => {
                return Poll::Ready(Err(e))
            },
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
        };
        let resp = match resp {
            Ok(resp) => resp,
            Err(e) => {
                let e = Error::new(ErrorKind::Other, e);
                return Poll::Ready(Err(e));
            },
        };
        Poll::Ready(Ok(resp))
    }
}
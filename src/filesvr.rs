use std::{
    io::{
        Error, 
        Result, 
        ErrorKind
    },
    pin::Pin, 
    task::{Poll, Context},
    result::Result as StdResult,
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

#[derive(Clone)]
pub struct FileService {
    local_root: String
}

impl FileService {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            local_root: root.into()
        }
    }

    pub async fn serv<B>(self, request: Request<B>) -> Result<Response<Body>> {
        let request_resolve = RequestResolve::new(&self.local_root, &request);
        let resolved = request_resolve.await?;
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

impl<B> Service<Request<B>> for FileService
where
    B: Sync + Send + 'static
{
    type Response = Response<Body>;

    type Error = Error;

    type Future = FileServiceFuture;

    fn call(&mut self, request: Request<B>) -> Self::Future {
        FileServiceFuture::new(&self.local_root, request)
    }

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

}

pub struct FileServiceFuture {
    req_resolve: RequestResolve,
    
}

impl FileServiceFuture {
    fn new<B>(local_root: &str, request: Request<B>) -> Self {
        Self {
            req_resolve: RequestResolve::new(local_root, &request),
        }
    }
}

impl Future for FileServiceFuture {
    type Output = Result<Response<Body>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self{
            ref mut req_resolve,
        } = *self;
        let resolved = match Pin::new(req_resolve).poll(cx) {
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

#[derive(Clone)]
pub struct FileServiceMaker {
    local_root: String
}

impl FileServiceMaker {
    pub fn new(local_root: impl Into<String>) -> Self {
        Self {
            local_root: local_root.into()
        }
    }
}

impl<T> Service<T> for FileServiceMaker {
    type Response = FileService;

    type Error = hyper::Error;

    type Future =  Pin<Box<dyn Future<Output = StdResult<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<StdResult<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        let local_root = self.local_root.clone();
        Box::pin(async move { Ok(FileService::new(local_root)) })
    }
}
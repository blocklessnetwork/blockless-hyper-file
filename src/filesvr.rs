use std::{
    io::{Error, Result},
    future::Future, 
    pin::Pin, 
    task::{Poll, Context}
};

use hyper::{
    service::Service, 
    Request, 
    Response, StatusCode
};

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

impl FileSvr {
  
    async fn serv<B>(self, req: Request<B>) -> Result<Response<Body>> {
        let resolved = RequestResolve::new(&self.local_root, &req).await?;
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
            Resolved::Found(f) => {
                ResponseBuilder::new().build(f)
            },
        }.unwrap();
        Ok(resp)
    }
}



impl<B> Service<Request<B>> for FileSvr 
where
    B: Sync + Send + 'static
{
    type Response = Response<Body>;

    type Error = Error;

    type Future = FileRespone;

    fn call(&mut self, req: Request<B>) -> Self::Future {
        todo!()
    }

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub struct FileRespone {
    body: Response<Body>
}

impl Future for FileRespone {
    type Output = Result<Response<Body>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!()
    }
}
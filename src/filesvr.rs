use std::{io::Error as IoError, future::Future, pin::Pin, task::{Poll, Context}};

use hyper::{
    service::Service, 
    Request, 
    Response
};

use crate::body::Body;

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
  
    async fn serv<B>(self, req: Request<B>) -> Result<Response<Body>, IoError> {
        let path = req.uri().path();
        
        Ok(Response::new(Body{}))
    }
}



impl<B> Service<Request<B>> for FileSvr 
where
    B: Sync + Send + 'static
{
    type Response = Response<Body>;

    type Error = IoError;

    type Future = FileRespone;

    fn call(&mut self, req: Request<B>) -> Self::Future {
        
    }

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub(crate) struct FileRespone {
    body: Response<Body>
}

impl Future for FileRespone {
    type Output = Result<Response<Body>, IoError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Ok(self.body))
    }
}
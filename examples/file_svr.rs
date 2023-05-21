use std::{net::SocketAddr, convert::Infallible};

use hyper_file::FileSvr;
use tokio::runtime::Builder;
use hyper::{Server, service::make_service_fn};

fn main() {
    let rt = Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();
    let addr: SocketAddr = ([127, 0, 0, 1], 9088).into();
    let make_svc = make_service_fn(|_conn| {
        async { Ok::<_, Infallible>(FileSvr::new("./")) }
    });
    rt.block_on(async move {
        let builder = Server::bind(&addr);
        let server = builder.serve(make_svc);
        server.await.unwrap();
    });
}
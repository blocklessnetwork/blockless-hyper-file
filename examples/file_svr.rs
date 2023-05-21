use std::net::SocketAddr;

use hyper_file::FileServiceMaker;
use tokio::runtime::Builder;
use hyper::Server;

fn main() {
    let rt = Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();
    let addr: SocketAddr = ([127, 0, 0, 1], 9088).into();
    rt.block_on(async move {
        let builder = Server::bind(&addr);
        let server = builder.serve(FileServiceMaker::new("/Users/join/Downloads/hyper-file/"));
        server.await.unwrap();
    });
}
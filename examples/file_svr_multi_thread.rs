use std::net::SocketAddr;

use blockless_hyper_file::FileServiceMaker;
use hyper::Server;
use tokio::runtime::Builder;

fn main() {
    let rt = Builder::new_multi_thread().enable_io().build().unwrap();
    let addr: SocketAddr = ([127, 0, 0, 1], 9088).into();
    rt.block_on(async move {
        let builder = Server::bind(&addr);
        let server = builder.serve(FileServiceMaker::new("."));
        server.await.unwrap();
    });
}

# Blockless hyper-file

This crate is implemention of  the http file range for hyper extension. 

## Easy use.

```rust
let builder = Server::bind(&addr);
let server = builder.serve(FileServiceMaker::new("."));
server.await.unwrap();
```




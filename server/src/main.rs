use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use tokio::fs;

#[tokio::main]
async fn main() {
    let addr = ([127, 0, 0, 1], 8080).into();
    let service = make_service_fn(|_| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(service);

    println!("Server running on http://{}", addr);
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}


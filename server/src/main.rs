use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use tokio::fs;
use std::path::Path;

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            Ok(Response::new(Body::from("Welcome to the Secure Rust Server!")))
        }
        (&Method::GET, path) => {
            let file_path = format!(".{}", path);
            if Path::new(&file_path).exists() {
                match fs::read_to_string(file_path).await {
                    Ok(contents) => Ok(Response::new(Body::from(contents))),
                    Err(_) => Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Error reading file"))
                        .unwrap()),
                }
            } else {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("404 - Not Found"))
                    .unwrap())
            }
        }
        _ => {
            Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::from("405 - Method Not Allowed"))
                .unwrap())
        }
    }
}

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


use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use tokio::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncWriteExt;

// Define a struct for form data
#[derive(Deserialize, Serialize)]
struct FormData {
    name: String,
    email: String,
    message: String,
}

// Function to handle requests
async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        // GET request for the homepage
        (&Method::GET, "/") => {
            Ok(Response::new(Body::from("Welcome to the Secure Rust Server!")))
        }

        // Serve static files
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

        // Handle POST request for form submission
        (&Method::POST, "/submit") => {
            let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

            // Deserialize JSON form data
            match serde_json::from_slice::<FormData>(&whole_body) {
                Ok(form_data) => {
                    let response_msg = format!("Received: Name: {}, Email: {}, Message: {}", 
                                               form_data.name, form_data.email, form_data.message);
                    
                    // Save form data to a file
                    let mut file = tokio::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open("form_submissions.txt")
                        .await
                        .unwrap();
                    file.write_all(format!("{:?}\n", form_data).as_bytes()).await.unwrap();

                    Ok(Response::new(Body::from(response_msg)))
                }
                Err(_) => {
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("Invalid form data"))
                        .unwrap())
                }
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


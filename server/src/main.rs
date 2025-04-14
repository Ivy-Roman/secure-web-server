use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use tokio::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use regex::Regex;
use log::{info, warn, error};

// Define a struct for form data
#[derive(Deserialize, Serialize, Debug)]
struct FormData {
    name: String,
    email: String,
    message: String,
}

impl FormData {
    fn is_valid(&self) -> Result<(), String> {
        // Check for empty fields
        if self.name.trim().is_empty()
            || self.email.trim().is_empty()
            || self.message.trim().is_empty()
        {
            return Err("All fields are required.".into());
        }

        // Check for max lengths
        if self.name.len() > 100 {
            return Err("Name is too long (max 100 characters).".into());
        }
        if self.email.len() > 100 {
            return Err("Email is too long (max 100 characters).".into());
        }
        if self.message.len() > 1000 {
            return Err("Message is too long (max 1000 characters).".into());
        }

        // Validate email format
        let email_regex =
            Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
        if !email_regex.is_match(&self.email) {
            return Err("Invalid email format.".into());
        }

        // Optional: Block basic script injection patterns
        if self.message.contains("<script>") {
            return Err("Message contains potentially unsafe content.".into());
        }

        Ok(())
    }
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        // Handle GET requests (serve static files)
        (&Method::GET, path) => {
            let safe_path = sanitize_path(path);

            match safe_path {
                Some(path) => {
                    match fs::read_to_string(path).await {
                        Ok(contents) => Ok(Response::new(Body::from(contents))),
                        Err(_) => Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::from("404 - File not found"))
                            .unwrap()),
                    }
                }
                None => Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("400 - Invalid path"))
                    .unwrap()),
            }
        }

        // Handle POST form submissions
        (&Method::POST, "/submit") => {
            let full_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

            match serde_json::from_slice::<FormData>(&full_body) {
                Ok(form_data) => {
                    if let Err(msg) = form_data.is_valid() {
    return Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from(msg))
        .unwrap());
}

                    }

                    let mut file = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("form_submissions.txt")
                        .await
                        .unwrap();

                    let log_entry = format!("{:?}\n", form_data);
                    file.write_all(log_entry.as_bytes()).await.unwrap();

                    Ok(Response::new(Body::from("Thank you! Your message was received.")))
                }
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Invalid form submission"))
                    .unwrap()),
            }
        }

        // Handle all other requests
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 - Not Found"))
                .unwrap())
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize logger before anything else
    env_logger::init();
    info!("Server starting on http://127.0.0.1:8080");

    // Define server address
    let addr = ([127, 0, 0, 1], 8080).into();

    // Create the service using the request handler
    let service = make_service_fn(|_| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    // Start the Hyper server
    let server = Server::bind(&addr).serve(service);

    println!("Server running on http://{}", addr);

    // Await the server and handle any fatal errors
    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }
}



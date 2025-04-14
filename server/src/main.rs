use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use tokio::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use regex::Regex;
use log::{info, warn, error};
use env_logger;
use std::os::unix::fs::OpenOptionsExt;

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

        // Validate email format with regex
        let email_regex = Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
        if !email_regex.is_match(&self.email) {
            return Err("Invalid email format.".into());
        }

        // Basic XSS protection
        if self.message.contains("<script>") {
            return Err("Message contains potentially unsafe content.".into());
        }

        Ok(())
    }
}

// Sanitize path to avoid directory traversal
fn sanitize_path(request_path: &str) -> Option<std::path::PathBuf> {
    let rel_path = if request_path == "/" {
        "static/form.html"
    } else {
        &request_path[1..]
    };

    let path = Path::new("static").join(rel_path);
    if path.exists() && path.starts_with("static") {
        Some(path)
    } else {
        None
    }
}

// Main request handler
async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let method = req.method();
    let uri = req.uri().path();
    info!("Incoming request: {} {}", method, uri);

    match (method, uri) {
        // Serve static files
        (&Method::GET, path) => {
            let safe_path = sanitize_path(path);
            match safe_path {
                Some(path) => match fs::read_to_string(path).await {
                    Ok(contents) => Ok(Response::new(Body::from(contents))),
                    Err(e) => {
                        error!("Failed to read file: {:?}", e);
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("500 - Internal Server Error"))
                            .unwrap())
                    }
                },
                None => {
                    warn!("Attempted to access invalid path: {}", path);
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("400 - Invalid path"))
                        .unwrap())
                }
            }
        }

        // Handle form submission
        (&Method::POST, "/submit") => {
            let full_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

            match serde_json::from_slice::<FormData>(&full_body) {
                Ok(form_data) => {
                    info!("Parsed form data: {:?}", form_data);

                    if let Err(msg) = form_data.is_valid() {
                        warn!("Validation error: {}", msg);
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from(msg))
                            .unwrap());
                    }

                    let mut file = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .mode(0o600)
                        .open("form_submissions.txt")
                        .await
                        .unwrap_or_else(|e| {
                            error!("Failed to open file: {:?}", e);
                            panic!();
                        });

                    let log_entry = format!("{:?}\n", form_data);
                    if let Err(e) = file.write_all(log_entry.as_bytes()).await {
                        error!("Failed to write to file: {:?}", e);
                    } else {
                        info!("Successfully saved submission for {}", form_data.email);
                    }

                    Ok(Response::new(Body::from("Thank you! Your message was received.")))
                }
                Err(e) => {
                    warn!("Failed to parse JSON body: {:?}", e);
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("Invalid form submission"))
                        .unwrap())
                }
            }
        }

        // All other requests
        _ => {
            warn!("Unknown route requested: {}", uri);
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 - Not Found"))
                .unwrap())
        }
    }
}

// Entry point
#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Secure web server starting...");

    let addr = ([127, 0, 0, 1], 8080).into();

    let service = make_service_fn(|_| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    let server = Server::bind(&addr).serve(service);

    println!("Server running on http://{}", addr);

    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }
}


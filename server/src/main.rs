// Import core dependencies for building the async HTTP server
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;

// Async file operations
use tokio::fs;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use std::os::unix::fs::OpenOptionsExt;

// For form data serialization/deserialization
use serde::{Deserialize, Serialize};

// Regex for advanced email validation
use regex::Regex;

// Structured logging macros
use log::{info, warn, error};
use env_logger;

// Used to set custom HTTP response headers
use hyper::header::{HeaderValue, CONTENT_TYPE};

/// Data structure representing the form submission fields.
/// This struct is automatically serialized/deserialized from JSON.
#[derive(Deserialize, Serialize, Debug)]
struct FormData {
    name: String,
    email: String,
    message: String,
}

impl FormData {
    /// Validate form fields for correctness, length, format, and safety.
    fn is_valid(&self) -> Result<(), String> {
        // Check required fields
        if self.name.trim().is_empty()
            || self.email.trim().is_empty()
            || self.message.trim().is_empty()
        {
            return Err("All fields are required.".into());
        }

        // Enforce length limits to prevent abuse
        if self.name.len() > 100 {
            return Err("Name is too long (max 100 characters).".into());
        }

        if self.email.len() > 100 {
            return Err("Email is too long (max 100 characters).".into());
        }

        if self.message.len() > 1000 {
            return Err("Message is too long (max 1000 characters).".into());
        }

        // Validate proper email format using regex
        let email_regex = Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
        if !email_regex.is_match(&self.email) {
            return Err("Invalid email format.".into());
        }

        // Basic script injection protection
        if self.message.contains("<script>") {
            return Err("Message contains potentially unsafe content.".into());
        }

        Ok(())
    }
}

/// Adds standard HTTP security headers to the response to prevent attacks.
fn with_security_headers(mut response: Response<Body>) -> Response<Body> {
    let headers = response.headers_mut();
    headers.insert("Content-Security-Policy", HeaderValue::from_static("default-src 'self'"));
    headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    response
}

/// Sanitizes the requested path to prevent directory traversal attacks.
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

/// Core HTTP request handler for both GET and POST requests.
async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let method = req.method();
    let uri = req.uri().path();
    info!("Incoming request: {} {}", method, uri);

    match (method, uri) {
        // Serve static files for GET requests
        (&Method::GET, path) => {
            let safe_path = sanitize_path(path);
            match safe_path {
                Some(path) => match fs::read_to_string(path).await {
                    Ok(contents) => {let response = Response::builder()
    .status(StatusCode::OK)
    .header("Content-Type", "text/html; charset=utf-8")
    .body(Body::from(contents))
    .unwrap();

Ok(with_security_headers(response))},
                    Err(e) => {
                        error!("Failed to read file: {:?}", e);
                        Ok(with_security_headers(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("500 - Internal Server Error"))
                            .unwrap()))
                    }
                },
                None => {
                    warn!("Attempted to access invalid path: {}", path);
                    Ok(with_security_headers(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("400 - Invalid path"))
                        .unwrap()))
                }
            }
        }

        // Handle form submissions via POST
        (&Method::POST, "/submit") => {
            // Check content-type header
            if req.headers().get("content-type") != Some(&"application/json".parse().unwrap()) {
                warn!("Unsupported content type: {:?}", req.headers().get("content-type"));
                return Ok(with_security_headers(Response::builder()
                    .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
                    .body(Body::from("Expected application/json"))
                    .unwrap()));
            }

            // Read and limit the body size to 10KB
            let full_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let max_size = 10 * 1024;
            if full_body.len() > max_size {
                warn!("Rejected large payload: {} bytes", full_body.len());
                return Ok(with_security_headers(Response::builder()
                    .status(StatusCode::PAYLOAD_TOO_LARGE)
                    .body(Body::from("Payload too large"))
                    .unwrap()));
            }

            // Attempt to parse JSON body
            match serde_json::from_slice::<FormData>(&full_body) {
                Ok(form_data) => {
                    info!("Parsed form data: {:?}", form_data);

                    // Validate input fields
                    if let Err(msg) = form_data.is_valid() {
                        warn!("Validation error: {}", msg);
                        return Ok(with_security_headers(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from(msg))
                            .unwrap()));
                    }

                    // Save submission securely to file
                    let mut file = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .mode(0o600) // Secure file permissions (rw-------)
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

                    Ok(with_security_headers(Response::new(Body::from(
                        "Thank you! Your message was received.",
                    ))))
                }

                Err(e) => {
                    warn!("Failed to parse JSON body: {:?}", e);
                    Ok(with_security_headers(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("Invalid form submission"))
                        .unwrap()))
                }
            }
        }

        // Reject all other methods and paths
        _ => {
            warn!("Unknown route requested: {}", uri);
            Ok(with_security_headers(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 - Not Found"))
                .unwrap()))
        }
    }
}

/// The application's entry point. Initializes logging and starts the server.
#[tokio::main]
async fn main() {
    // Start structured logging (info, warn, error)
    env_logger::init();
    info!("Secure web server starting...");

    // Bind server to localhost:8080
    let addr = ([127, 0, 0, 1], 8080).into();

    // Create the Hyper service from the request handler
    let service = make_service_fn(|_| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    // Run the server
    let server = Server::bind(&addr).serve(service);

    println!("Server running on http://{}", addr);

    // Graceful shutdown with error logging
    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }
}

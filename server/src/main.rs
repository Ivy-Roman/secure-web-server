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
        // Serve static files securely
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

        // Handle form submission via POST
        (&Method::POST, "/submit") => {
            let full_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

            match serde_json::from_slice::<FormData>(&full_body) {
                Ok(form_data) => {
                    // Validate required fields
                    if form_data.name.trim().is_empty()
                        || form_data.email.trim().is_empty()
                        || form_data.message.trim().is_empty()
                    {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from("All fields are required."))
                            .unwrap());
                    }

                    // Validate email format
                    if !form_data.email.contains('@') {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from("Invalid email format."))
                            .unwrap());
                    }

                    // Save valid submission to file
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

                // Invalid JSON format
                Err(_) => Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Invalid form submission"))
                    .unwrap()),
            }
        }

        // Handle all other routes
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 - Not Found"))
                .unwrap())
        }
    }
}

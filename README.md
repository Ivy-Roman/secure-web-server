# Secure Web Server in Rust

This project implements a secure HTTP server in Rust. It is capable of serving static files and handling form submissions with proper validation, logging, and safe resource handling. It is built to run on Fedora Linux and aligns with system programming and security-focused software design principles.

## Features

- Handles HTTP `GET` and `POST` methods
- Serves `.html` files in a browser (Chrome, Firefox)
- Serves files from subdirectories under `static/`
- Accepts form submissions using JSON over `POST /submit`
- Validates input:
  - All fields required
  - Max length limits (name, email, message)
  - Regex-based email validation
  - Blocks script injection
- Stores submissions in `form_submissions.txt`
- Limits request body size to 10 KB
- Enforces `Content-Type: application/json`
- Logs info, warnings, and errors using `env_logger`
- Adds security headers:
  - `Content-Security-Policy`
  - `X-Content-Type-Options`
  - `X-Frame-Options`
- Sets secure file permissions (mode 600)
- Spawns isolated async tasks using `tokio`

## Folder Structure
.
├── src/
│   └── main.rs
├── static/
│   └── form.html
├── Cargo.toml
├── Cargo.lock
├── COMPILE_INSTRUCTIONS.txt
├── README.md
└── git_log_summary.txt

## How it Works
- Server runs on `127.0.0.1:8080`
- Visiting `/` or `/form.html` loads a contact form
- On submission, a JSON POST request is sent to `/submit`
- Input is validated, logged, and written to a local file
- Errors and invalid data are rejected with appropriate responses

## How to Build and Run

Ensure you have Rust and GCC installed (see `COMPILE_INSTRUCTIONS.txt` for setup on Fedora).

To build the project:

```bash
cargo build --release

## To run the server with logging:
RUST_LOG=info cargo run --release

Open your browser and visit:
http://127.0.0.1:8080

## Security Measures
- Input validation (required fields, length, format, XSS protection)
- Directory traversal protection
- Max body size enforcement
- MIME type enforcement
- Security headers to prevent browser attacks
- Logging for every request and error
- Isolated task handling via tokio::spawn
- File writes secured with restrictive permissions

## Limitations
- HTTPS is not implemented (as per project scope)
- Data is stored in plaintext for demonstration
- Not intended for production use without additional layers


Author [Ivie Osoiye]

/// The macro `create_response!` simplifies the creation of HTTP responses, accpepting status code, content type, and body.
macro_rules! create_response {
    ($status:expr, $content_type:expr, $body:expr) => {
        hyper::Response::builder()
            .status($status)
            .header("Content-Type", $content_type)
            .body(http_body_util::Full::new(hyper::body::Bytes::from($body)))
            .expect("Failed to create response")
    };
}

/// Creates an error response with the given status code and error message.
macro_rules! error_response {
    ($status:expr, $error_message:expr) => {
        create_response!(
            $status,
            "text/plain; charset=utf-8",
            format!("Error: {}", $error_message)
        )
    };
}

/// Creates a success response with the given content type and body.
macro_rules! success_response {
    ($content_type:expr, $buffer:expr) => {
        create_response!(hyper::StatusCode::OK, $content_type, $buffer)
    };
}

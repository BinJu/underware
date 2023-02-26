use http::{Request, Response};
use std::io::{Read, Error, ErrorKind, Result};
use crate::http_text;

pub trait StreamBuilder {
    type Output;
    fn build<R: Read>(r: &mut R) -> Result<Self::Output>;
}

pub struct RequestStreamBuilder {}

impl StreamBuilder for RequestStreamBuilder {
    type Output = Request<String>;
    fn build<R: Read>(r: &mut R) -> Result<Self::Output> {
        let version = http_text::parse_request_version(r)?;
        let header = http_text::parse_header(r)?;
        let body = http_text::parse_string_body(r, 0)?;
        let builder = http::request::Builder::new();
        let mut builder = builder.version(version.version);
        builder = builder.method(version.method);
        builder = builder.uri(version.uri);
        let headers = builder.headers_mut().unwrap();
        *headers = header;
        builder.body(body).map_err(|err|Error::new(ErrorKind::InvalidData, format!("Http body error: {}", err)))
    }
}

pub struct ResponseStreamBuilder {}
// TODO: error handling
impl StreamBuilder for ResponseStreamBuilder {
    type Output = Response<String>;
    fn build<R: Read>(r: &mut R) -> Result<Self::Output> {
        let version = http_text::parse_response_version(r)?;
        let header = http_text::parse_header(r)?;
        let body = http_text::parse_string_body(r, 0)?;
        let builder = http::response::Builder::new();
        let mut builder = builder.version(version.version);
        builder = builder.status(version.status);
        let headers = builder.headers_mut().unwrap();
        *headers = header;
        builder.body(body).map_err(|err|Error::new(ErrorKind::InvalidData, format!("Http body error: {}", err)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn test_response_build_from_tcp_stream() {
        let response_raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nage: 13360\r\n\r\n<html><body>Hello world</body></html>\r\n";
        let mut buff = BufReader::new(&response_raw[..]);
        let response = ResponseStreamBuilder::build(&mut buff).unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.version(), http::Version::HTTP_11);
        assert_eq!(response.headers().len(), 2);
    }
    #[test]
    fn test_request_build_from_tcp_stream() {
        // example: POST /home HTTP/1.1\r\nHOST: localhost:8080\r\nUser-Agent: Mozilla/5.0\r\nAccept: text/html, application/xhtml+xml,*/*;q=0.8\r\n\r\n<html><body>this is a test</body></html>\r\n
        let request_raw = b"POST /home HTTP/1.1\r\nHOST: localhost:8080\r\nUser-Agent: Mozilla/5.0\r\nAccept: text/html, application/xhtml+xml,*/*;q=0.8\r\n\r\n<html><body>this is a test</body></html>\r\n";
        let mut buff = BufReader::new(&request_raw[..]);
        let request = RequestStreamBuilder::build(&mut buff).unwrap();
        assert_eq!(request.method(), http::Method::POST);
        assert_eq!(request.version(), http::Version::HTTP_11);
        assert_eq!(request.uri(), "/home");
        let headers = request.headers();
        assert_eq!(headers.len(), 3);
        assert_eq!(headers["HOST"], "localhost:8080");
        assert_eq!(headers["User-Agent"], "Mozilla/5.0");
        assert_eq!(headers["Accept"], "text/html, application/xhtml+xml,*/*;q=0.8");
    }
}

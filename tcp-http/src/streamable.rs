use http::{Request, Response, HeaderMap};
use std::io::{Write, Error, ErrorKind, Result};

pub trait Streamable {
    fn write<W: Write>(&self, w: &mut W) -> Result<()>;
}

impl <T>Streamable for Request<T> where T: AsRef<str> {
    fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        serialize_http_11(|| format!("{} {} {:?}\r\n", self.method(), self.uri(), self.version()), self.headers(), self.body().as_ref(), w)
    }
}

impl <T>Streamable for Response<T> where T: ToString{
    fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        serialize_http_11(|| format!("{:?} {}\r\n", self.version(), self.status()), self.headers(), &*self.body().to_string(), w)
    }
}

fn serialize_http_11<W: Write, F: FnOnce() -> String>(version_line: F, headers: &HeaderMap, body: &str, w: &mut W) -> Result<()> {
    let mut text = version_line();
    for (k,v) in headers {
        text.push_str(k.as_str());
        text.push_str(": ");
        let data = v.to_str().map_err(|err| Error::new(ErrorKind::InvalidData, format!("failed to convert header data to string: {}", err)))?;
        text.push_str(data);
        text.push_str("\r\n");
    }
    text.push_str("\r\n");
    text.push_str(body);
    w.write_all(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use http::{Method, Version};
    use http::response::Builder as ResponseBuilder;
    use http::request::Builder as RequestBuilder;

    use super::*;
    #[test]
    fn test_write_response() {
        let builder = ResponseBuilder::new().status(200).version(Version::HTTP_11).header("Content-Type", "text/html").header("keep-alive", "true");
        let response = builder.body("<html><body>This is the body</body></html>").unwrap();
        let mut buf = Vec::<u8>::new();
        response.write(&mut buf).unwrap();
        let str_buff = String::from_utf8_lossy(&buf);
        assert_eq!(str_buff, "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\nkeep-alive: true\r\n\r\n<html><body>This is the body</body></html>");
    }

    #[test]
    fn test_write_request() {
        let builder = RequestBuilder::new().version(Version::HTTP_11).method(Method::POST).uri("/home").header("Content-Type", "text/html").header("keep-alive", "true");
        let response = builder.body("{\"name\":\"John\"}").unwrap();
        let mut buf = Vec::<u8>::new();
        response.write(&mut buf).unwrap();
        let str_buff = String::from_utf8_lossy(&buf);
        assert_eq!(str_buff, "POST /home HTTP/1.1\r\ncontent-type: text/html\r\nkeep-alive: true\r\n\r\n{\"name\":\"John\"}");
    }
}

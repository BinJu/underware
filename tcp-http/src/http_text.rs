use http::{Version, HeaderValue, HeaderMap, header::HeaderName, Method};
use std::io::{Read, Error, ErrorKind, Result};

pub struct HttpResponseVersion {
    pub version: Version,
    pub status: u16
}

pub struct HttpRequestVersion {
    pub version: Version,
    pub uri: String,
    pub method: Method
}


fn read_line<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<usize> {
    let mut idx:usize = 0;
    let mut ch_buff = [0u8;1];
    let mut new_line_flag = false;
    while let Ok(cnt) = r.read(&mut ch_buff) {
        if cnt != 1 { break }
        if idx + 1 >= buf.len() { return Err(Error::new(ErrorKind::OutOfMemory, "Insufficent buffer")) }
        match ch_buff[0] {
            b'\r' => new_line_flag = true,
            b'\n' => {if new_line_flag {break}},
            c => {buf[idx] = c; idx += 1}
        };
    }
    Ok(idx)
}

pub fn parse_request_version<R: Read>(r: &mut R) -> Result<HttpRequestVersion> {
    // example: POST / HTTP/1.1\r\nHOST: localhost:8080\r\nUser-Agent: Mozilla/5.0\r\nAccept: text/html, application/xhtml+xml,*/*;q=0.8\r\n\r\n<html><body>this is a test</body></html>\r\n
    let mut buff = [0u8;1024];
    let buff_len = read_line(r, &mut buff)?;
    let line = String::from_utf8_lossy(&buff[..buff_len]);
    let mut protocols = line.split(" ");

    let method = match protocols.next() {
        Some(str_method) => Method::from_bytes(str_method.as_bytes()).map_err(|_|Error::new(ErrorKind::InvalidInput, format!("Unknown http method: {}", str_method)))?,
        None => return Err(Error::new(ErrorKind::InvalidInput, "Failed to parse http version"))

    };
    let uri = if let Some(str_uri) = protocols.next() {
        String::from(str_uri)
    } else {
        return Err(Error::new(ErrorKind::InvalidInput, "Failed to parse http URI"));
    };

    let version = if let Some(str_version) = protocols.next() {
        version_from_str(str_version)?
    } else {
        return Err(Error::new(ErrorKind::InvalidInput, "Failed to parse http version"));
    };
    Ok(HttpRequestVersion { version, method, uri})
}

pub fn parse_response_version<R: Read>(r: &mut R) -> Result<HttpResponseVersion> {
    let mut buff = [0u8;1024];
    let buff_len = read_line(r, &mut buff)?;
    let line = String::from_utf8_lossy(&buff[..buff_len]);
    let mut protocols = line.split(" ");

    let version = if let Some(str_version) = protocols.next() {
        version_from_str(str_version)?
    } else {
        return Err(Error::new(ErrorKind::InvalidInput, "Failed to parse http version"));
    };

    let status = if let Some(str_status) = protocols.next() {
        str_status.parse::<u16>().map_err(|err| Error::new(ErrorKind::InvalidInput, format!("failed to parse http status: {}", err)))?
    } else {
        return Err(Error::new(ErrorKind::InvalidInput, "Failed to parse http version"));
    };

    Ok(HttpResponseVersion { version, status })
}

pub fn parse_header<R: Read>(input: &mut R) -> Result<HeaderMap> {
    let mut buff = [0u8;1024];
    let mut header = HeaderMap::new();
    loop {
        let buff_len = read_line(input, &mut buff)?;
        if buff_len == 0 { break }
        let line = String::from_utf8_lossy(&buff[..buff_len]);
        let sep = line.find(':');
        match sep {
            None => continue,
            Some(p) => {
                let k = &line[..p];
                let v = &line[p+1 ..];
                let v = v.trim();

                if let Ok(valid_value) = HeaderValue::from_str(v) {
                    let key_name = k.as_bytes();
                    match HeaderName::from_bytes(&key_name) {
                        Ok(header_name) => {header.insert(header_name, valid_value);},
                        Err(err) => { return Err(Error::new(ErrorKind::InvalidInput, format!("Invalid header name: {}", err))) }
                    };
                } else {
                    return Err(Error::new(ErrorKind::InvalidInput, format!("Invalid header value: {}", v)));
                }
            }
        }
    }
    Ok(header)
}

#[allow(dead_code)]
#[inline]
pub fn parse_body<R: Read>(input: &mut R, output: &mut [u8]) -> Result<usize> {
    input.read(output)
}

pub fn parse_string_body<R: Read>(input: &mut R, expect: usize) -> Result<String> {
    let mut result = String::with_capacity(expect);
    let mut temp = [0u8; 1024];
    let mut total_read = 0;
    loop {
        let read = input.read(&mut temp)?;
        if read == 0 { break }
        total_read += read;
        result.push_str(&String::from_utf8_lossy(&temp[..read]).to_string());
        if total_read >= expect { break }
    }
    
    Ok(result)
}

fn version_from_str(version_str: &str) -> Result<Version> {
    match version_str {
        "HTTP/0.9" => Ok(Version::HTTP_09),
        "HTTP/1.0" => Ok(Version::HTTP_10),
        "HTTP/1.1" => Ok(Version::HTTP_11),
        "HTTP/2.0" => Ok(Version::HTTP_2),
        "HTTP/3.0" => Ok(Version::HTTP_3),
        _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid HTTP Version: {}", version_str))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn test_parse_response_version_happy() {
        let input = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
        let mut buff = BufReader::new(&input[..]);
        let response_version = parse_response_version(&mut buff).unwrap();
        assert_eq!(response_version.version, Version::HTTP_11);
    }

    #[test]
    fn test_http_response_text_protocol_bad() {
        let input = b"HTTP/5.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
        let mut buff = BufReader::new(&input[..]);
        let response_version = parse_response_version(&mut buff);
        assert!(response_version.is_err());
        if let Err(err) = response_version {
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
            assert_eq!(err.to_string(), "Invalid HTTP Version: HTTP/5.1");
        }
    }

    #[test]
    fn test_http_response_text_code_happy() {
        let input = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
        let mut buff = BufReader::new(&input[..]);
        let response_version = parse_response_version(&mut buff).unwrap();
        assert_eq!(response_version.status, 200);
    }

    #[test]
    fn test_http_response_text_code_non_digital() {
        let input = b"HTTP/1.1 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
        let mut buff = BufReader::new(&input[..]);
        let response_version = parse_response_version(&mut buff);
        assert!(response_version.is_err());
        if let Err(err) = response_version {
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
            assert_eq!(err.to_string(), "failed to parse http status: invalid digit found in string");
        }
    }

    #[test]
    fn test_http_response_text_header_happy() {
        let input = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nage:33160\r\n\r\n<html><body>Hello world</body></html>\r\n";
        let mut buff = BufReader::new(&input[..]);
        let response_version = parse_response_version(&mut buff).unwrap();
        assert_eq!(response_version.status, 200);
        let headers = parse_header(&mut buff).unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers["Content-Type"], "text/html; charset=UTF-8");
        assert_eq!(headers["age"], "33160");
    }

    #[test]
    fn test_http_response_text_header_empty() {
        let input = b"HTTP/1.1 200 OK\r\n\r\n<html><body>Hello world</body></html>\r\n";
        let mut buff = BufReader::new(&input[..]);
        let response_version = parse_response_version(&mut buff).unwrap();
        assert_eq!(response_version.status, 200);
        let headers = parse_header(&mut buff).unwrap();
        assert_eq!(headers.len(), 0);
    }

    #[test]
    fn test_http_response_text_body() {
        let input = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nage:33160\r\n\r\n<html><body>Hello world</body></html>\r\n";
        let mut buff = BufReader::new(&input[..]);
        let response_version = parse_response_version(&mut buff).unwrap();
        assert_eq!(response_version.status, 200);
        let headers = parse_header(&mut buff).unwrap();
        assert_eq!(headers.len(), 2);
        let body = parse_string_body(&mut buff, 1024).unwrap();
        assert_eq!(body, "<html><body>Hello world</body></html>\r\n");

    }

    #[test]
    fn test_http_response_text_empty() {
        let input = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nage:33160\r\n\r\n";
        let mut buff = BufReader::new(&input[..]);
        let response_version = parse_response_version(&mut buff).unwrap();
        assert_eq!(response_version.status, 200);
        let headers = parse_header(&mut buff).unwrap();
        assert_eq!(headers.len(), 2);
        let body = parse_string_body(&mut buff, 1024).unwrap();
        assert_eq!(body, "");
    }

    #[test]
    fn test_read_line() {
        let input = b"This is line one\r\nThis is line two\r\nThis is line three.";
        let mut buff = BufReader::new(&input[..]);
        let mut out = [0u8; 1024];
        let len = read_line(&mut buff, &mut out).unwrap();
        assert_eq!(&out[..len], b"This is line one");

        let len = read_line(&mut buff, &mut out).unwrap();
        assert_eq!(&out[..len], b"This is line two");

        let len = read_line(&mut buff, &mut out).unwrap();
        assert_eq!(&out[..len], b"This is line three.");
    }
    #[test]
    fn test_read_line_empty() {
        let input = b"This is line one\r\n\r\n";
        let mut buff = BufReader::new(&input[..]);
        let mut out = [0u8; 1024];
        let len = read_line(&mut buff, &mut out).unwrap();
        assert_eq!(&out[..len], b"This is line one");

        let len = read_line(&mut buff, &mut out).unwrap();
        assert_eq!(len, 0);
    }

    #[test]
    fn test_parse_string_body() {
        let input = b"This is line one\r\n\r\n";
        let mut buff = BufReader::new(&input[..]);
        let body = parse_string_body(&mut buff, 64).unwrap();
        assert_eq!(body, "This is line one\r\n\r\n");
    }

    #[test]
    fn test_parse_string_body_empty() {
        let input = b"";
        let mut buff = BufReader::new(&input[..]);
        let body = parse_string_body(&mut buff, 64).unwrap();
        assert_eq!(body, "");
    }
}

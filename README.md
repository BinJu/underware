## Why this project
This project is practising bridging between TCP Stream and HttpRequest, HttpResponse.
The library create HttpRequest from the tcp packets and build the HttpResponse object, later serialize to Tcp Stream.

Please see the below example below and you may find it in `sample` sub project.

```rust
use tcp_http::{Streamable, StreamBuilder, RequestStreamBuilder};

fn handle_client(req: Request<String>, res: &mut Response<String>) {
    let uri = req.uri();
    println!("[DEBUG]: URI={}", uri);
    let req_body = req.body();
    let status = res.status_mut();
    *status = StatusCode::from_u16(200).unwrap();
    let res_body = res.body_mut();
    *res_body = req_body.to_string();
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8081").unwrap();
    println!("Listening for connections on port {}", 8081);

    for stream in listener.incoming() {
        match stream {
            Ok(mut input) => {
                thread::spawn(move || {
                    let request = RequestStreamBuilder::build(&mut input).unwrap();
                    let mut response = http::response::Builder::new().body(String::new()).unwrap();
                    handle_client(request, &mut response);
                    response.write(&mut input).unwrap();
                });
            }
            Err(e) => {
                println!("Unable to connect: {}", e);
            }
        }
    }
}
```

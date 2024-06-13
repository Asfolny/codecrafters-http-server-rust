// Uncomment this block to pass the first stage
use std::net::{Shutdown, TcpStream, TcpListener};
use std::io::{Write, Read};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                let mut input = [0; 512];
                let _ = _stream.read(&mut input[..]);
                let parsed = input.map(|x| char::from(x));
                let mut path = String::new();
                let mut path_start = false;

                for chr in parsed {
                    if !path_start && chr != ' ' {
                        continue;
                    }

                    if !path_start {
                        path_start = true;
                        continue;
                    }

                    if chr == ' ' {
                        break;
                    }

                    path.push(chr);
                }

                let path_parts: Vec<&str> = path.as_str().split('/').collect();

                match path_parts[1] {
                    "" => handle_index(_stream),
                    "echo" => handle_echo(_stream, path_parts),
                    _ => handle_not_found(_stream)
                }
            },
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_index(mut stream: TcpStream) {
    let _ = stream.write(b"HTTP/1.1 200 OK\r\n\r\n");
    let _ = stream.shutdown(Shutdown::Both).expect("shutdown call failed");
}

fn handle_echo(mut stream: TcpStream, path_parts: Vec<&str>) {
    let mut body = String::new(); // placeholder to handle /echo without anything to echo
    
    if path_parts.len() == 3 { // /echo/asd handling
        body.push_str(path_parts[2]);
    }

    let _ = stream.write(b"HTTP/1.1 200 OK\r\n");
    let _ = stream.write(b"Content-Type: text/plain\r\n");
    let _ = stream.write(b"Content-Length: ");
    let _ = stream.write(body.len().to_string().as_bytes());
    let _ = stream.write(b"\r\n\r\n");
    let _ = stream.write(body.as_bytes());
    let _ = stream.shutdown(Shutdown::Both).expect("shutdown call failed");

}

fn handle_not_found(mut stream: TcpStream) {
    let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n");
    let _ = stream.shutdown(Shutdown::Both).expect("shutdown call failed");
}

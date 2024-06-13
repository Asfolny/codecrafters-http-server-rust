// Uncomment this block to pass the first stage
use std::net::{Shutdown, TcpListener};
use std::io::Write;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                let _ = _stream.write(b"HTTP/1.1 200 OK\r\n\r\n");
                let _ = _stream.shutdown(Shutdown::Both).expect("shutdown call failed");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

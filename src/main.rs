// Uncomment this block to pass the first stage
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use flate2::write::GzEncoder;
use flate2::Compression;

const SUPPORTED_ENCODINGS: [&str; 1] = ["gzip"];

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                thread::spawn(|| {
                    println!("accepted new connection");
                    let mut input = [0; 512];
                    let _ = _stream.read(&mut input[..]);
                    let parsed = input.map(|x| char::from(x));
                    let mut headers: HashMap<String, String> = HashMap::new();
                    let mut request_body = String::new();
                    let mut path = String::new();
                    let mut path_start = false;
                    let mut method = String::new();

                    for chr in parsed {
                        if !path_start && chr != ' ' {
                            method.push(chr);
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

                    let mut past_request_line = false;
                    let mut past_header_name = false;
                    let mut header_name = String::new();
                    let mut header_body = String::new();
                    let mut found_reset = false;
                    let mut reset_count = 0;
                    let mut body_start = false;
                    for chr in parsed {
                        if !past_request_line && !found_reset && chr != '\r' && chr != '\n' {
                            continue;
                        }

                        if !past_request_line && chr == '\r' {
                            found_reset = true;
                            continue;
                        }

                        if !past_request_line && chr == '\n' {
                            past_request_line = true;
                            found_reset = false; // reset, we need this for later
                            continue;
                        }

                        if !past_header_name
                            && chr != ':'
                            && chr != '\r'
                            && chr != '\n'
                            && !(found_reset && reset_count == 1)
                        {
                            reset_count = 0;
                            header_name.push(chr);
                            continue;
                        }

                        if reset_count == 1 && chr == '\r' {
                            found_reset = true;
                            continue;
                        }

                        if reset_count == 1 && found_reset == true && chr == '\n' {
                            body_start = true;
                            continue;
                        }

                        if body_start {
                            request_body.push(chr);
                            continue; // this will hoover up all the rest of the request
                        }

                        if !past_header_name {
                            past_header_name = true;
                            continue;
                        }

                        if !found_reset && chr != '\r' {
                            reset_count = 0;
                            header_body.push(chr);
                            continue;
                        }

                        if chr == '\r' {
                            found_reset = true;
                        }

                        if chr == '\n' {
                            // reset header parsing
                            found_reset = false;
                            past_header_name = false;
                            header_name = header_name.trim().to_string();
                            header_body = header_body.trim().to_string();
                            headers.insert(header_name, header_body);
                            header_name = String::new();
                            header_body = String::new();
                            reset_count = 1;
                        }
                    }

                    let path_parts: Vec<&str> = path.as_str().split('/').collect();

                    match method.as_str() {
                        "GET" => match path_parts[1] {
                            "" => handle_index(_stream),
                            "echo" => handle_echo(_stream, path_parts, headers),
                            "user-agent" => handle_user_agent(_stream, headers),
                            "files" => handle_file(_stream, path_parts),
                            _ => handle_not_found(_stream),
                        },
                        "POST" => match path_parts[1] {
                            "files" => handle_post_file(_stream, path_parts, request_body, headers),
                            _ => handle_not_found(_stream),
                        },
                        _ => handle_not_found(_stream),
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_index(mut stream: TcpStream) {
    let _ = stream.write(b"HTTP/1.1 200 OK\r\n\r\n");
    let _ = stream
        .shutdown(Shutdown::Both)
        .expect("shutdown call failed");
}

fn handle_echo(mut stream: TcpStream, path_parts: Vec<&str>, headers: HashMap<String, String>) {
    let mut body = String::new(); // placeholder to handle /echo without anything to echo
    let mut valid_encoding = false;
    let mut chosen_encoding = String::new();

    if path_parts.len() == 3 {
        // /echo/asd handling
        body.push_str(path_parts[2]);
    }

    if headers.contains_key("Accept-Encoding") {
        let header_encoding = headers
            .get("Accept-Encoding")
            .expect("Content-Encoding is needed here");

        let acceptable: Vec<&str> = header_encoding.split(", ").collect();

        if acceptable.len() == 1 {
            chosen_encoding = acceptable
                .get(0)
                .expect("Must have at least 1 encoding in header")
                .to_string();

            for enc in SUPPORTED_ENCODINGS {
                if enc == chosen_encoding {
                    valid_encoding = true;
                }
            }
        } else {
            'enc_option_loop: for option in acceptable {
                chosen_encoding = option.to_string();
                for enc in SUPPORTED_ENCODINGS {
                    if enc == chosen_encoding {
                        valid_encoding = true;
                        break 'enc_option_loop;
                    }
                }
            }
        }
    }

    let _ = stream.write(b"HTTP/1.1 200 OK\r\n");
    if valid_encoding {
        let _ = stream.write(b"Content-Encoding: ");
        let _ = stream.write(chosen_encoding.as_bytes());
        let _ = stream.write(b"\r\n");
    }
    let _ = stream.write(b"Content-Type: text/plain\r\n");
    if valid_encoding {
        let mut encoded_body = Vec::new();
        let _ = GzEncoder::new(&mut encoded_body, Compression::default()).write_all(body.as_bytes());
        let _ = stream.write(b"Content-Length: ");
        let _ = stream.write(encoded_body.len().to_string().as_bytes());
        let _ = stream.write(b"\r\n\r\n");
        let _ = stream.write(&encoded_body);
    } else {
        let _ = stream.write(b"Content-Length: ");
        let _ = stream.write(body.len().to_string().as_bytes());
        let _ = stream.write(b"\r\n\r\n");
        let _ = stream.write(body.as_bytes());
    }

    let _ = stream
        .shutdown(Shutdown::Both)
        .expect("shutdown call failed");
}

fn handle_user_agent(mut stream: TcpStream, headers: HashMap<String, String>) {
    let mut body = &String::new();

    if headers.contains_key("User-Agent") {
        body = headers.get("User-Agent").unwrap();
    }

    let _ = stream.write(b"HTTP/1.1 200 OK\r\n");
    let _ = stream.write(b"Content-Type: text/plain\r\n");
    let _ = stream.write(b"Content-Length: ");
    let _ = stream.write(body.len().to_string().as_bytes());
    let _ = stream.write(b"\r\n\r\n");
    let _ = stream.write(body.as_bytes());
    let _ = stream
        .shutdown(Shutdown::Both)
        .expect("shutdown call failed");
}

fn handle_post_file(
    mut stream: TcpStream,
    path_parts: Vec<&str>,
    input_body: String,
    headers: HashMap<String, String>,
) {
    if path_parts.len() == 3 {
        let args = env::args();
        let mut file_dir = String::new();
        let mut found_dir_option = false;

        for arg in args {
            if arg == "--directory" {
                found_dir_option = true;
                continue;
            }

            if found_dir_option {
                file_dir.push_str(arg.as_str());
                found_dir_option = false;
                break;
            }
        }

        file_dir.push_str(path_parts[2]);

        let mut file_content = String::new();
        let content_length: usize = match headers.get("Content-Length") {
            Some(length) => length.parse().expect("Content-Length MUST be a number"),
            None => 0,
        };

        if content_length > 0 {
            file_content.push_str(
                input_body
                    .get(0..content_length)
                    .expect("Body did not contain up to Content-Length"),
            );
        }

        let _ = fs::write(file_dir, file_content.as_str());
    }

    let _ = stream.write(b"HTTP/1.1 201 Created\r\n\r\n");
    let _ = stream
        .shutdown(Shutdown::Both)
        .expect("shutdown call failed");
}

fn handle_file(mut stream: TcpStream, path_parts: Vec<&str>) {
    let mut body = String::new();

    if path_parts.len() == 3 {
        let args = env::args();
        let mut file_dir = String::new();
        let mut found_dir_option = false;

        for arg in args {
            if arg == "--directory" {
                found_dir_option = true;
                continue;
            }

            if found_dir_option {
                file_dir.push_str(arg.as_str());
                found_dir_option = false;
                break;
            }
        }

        file_dir.push_str(path_parts[2]);
        body = match fs::read_to_string(file_dir) {
            Ok(file) => file,
            Err(_) => {
                println!("File does not exist");
                String::new()
            }
        };
    }

    if body.len() < 1 {
        let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n");
        stream
            .shutdown(Shutdown::Both)
            .expect("Shutdown call failed");
        return;
    }

    let _ = stream.write(b"HTTP/1.1 200 OK\r\n");
    let _ = stream.write(b"Content-Type: application/octet-stream\r\n");
    let _ = stream.write(b"Content-Length: ");
    let _ = stream.write(body.len().to_string().as_bytes());
    let _ = stream.write(b"\r\n\r\n");
    let _ = stream.write(body.as_bytes());
    let _ = stream
        .shutdown(Shutdown::Both)
        .expect("shutdown call failed");
}

fn handle_not_found(mut stream: TcpStream) {
    let _ = stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n");
    let _ = stream
        .shutdown(Shutdown::Both)
        .expect("shutdown call failed");
}

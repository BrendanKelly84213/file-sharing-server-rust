use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;

// Press Shift twice to open the Search Everywhere dialog and type `show whitespaces`,
// then press Enter. You can now see whitespace characters in your code.
fn main() {
    // Socket Client
    println!("Hello and welcome!");
    let stdin = io::stdin();

    match TcpStream::connect("localhost:12345") {
        Ok(client_socket) => {
            println!("Connected to the server at localhost:12345");

            let mut writer = match client_socket.try_clone() {
                Ok(socket) => socket,
                Err(e) => {
                    eprintln!("{e:?}");
                    return;
                }
            };
            let mut reader = BufReader::new(client_socket);

            loop {
                // Read user input from the console.
                let mut input = String::new();
                match stdin.lock().read_line(&mut input) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("{e:?}");
                        break;
                    }
                }

                let user_input = input.trim_end_matches(['\r', '\n']);
                if user_input.is_empty()
                    || user_input.eq_ignore_ascii_case("exit")
                    || user_input.eq_ignore_ascii_case("quit")
                {
                    break;
                }

                if let Err(e) = writeln!(writer, "{user_input}") {
                    eprintln!("{e:?}");
                    break;
                }
                if let Err(e) = writer.flush() {
                    eprintln!("{e:?}");
                    break;
                }
                println!("Message sent to the server: {user_input}");

                // Get the response.
                let mut response = String::new();
                match reader.read_line(&mut response) {
                    Ok(0) => println!("Response from server: null"),
                    Ok(_) => {
                        let response = response.trim_end_matches(['\r', '\n']);
                        println!("Response from server: {response}");
                    }
                    Err(e) => {
                        eprintln!("{e:?}");
                        break;
                    }
                }
            }

            println!("Connection closed.");
        }
        Err(e) => eprintln!("{e:?}"),
    }
}

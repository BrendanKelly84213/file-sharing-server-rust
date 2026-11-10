use crate::filesystem::FileSystemManager;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

pub struct FileServer {
    fs_manager: FileSystemManager,
    port: u16,
}

impl FileServer {
    pub fn new(port: u16, file_system_name: &str, total_size: usize) -> Self {
        // Initialize the FileSystemManager
        let fs_manager = FileSystemManager::new(file_system_name, total_size)
            .expect("FileSystemManager could not be initialized.");
        Self { fs_manager, port }
    }

    pub fn start(&self) {
        let server_socket = match TcpListener::bind(("0.0.0.0", self.port)) {
            Ok(socket) => socket,
            Err(e) => {
                eprintln!("{e:?}");
                eprintln!("Could not start server on port {}", self.port);
                return;
            }
        };

        println!("Server started. Listening on port {}...", self.port);

        for client_socket in server_socket.incoming() {
            match client_socket {
                Ok(client_socket) => {
                    println!("Handling client: {client_socket:?}");
                    if self.handle_client(client_socket) {
                        return;
                    }
                }
                Err(e) => eprintln!("{e:?}"),
            }
        }
    }

    fn handle_client(&self, client_socket: TcpStream) -> bool {
        let mut writer = match client_socket.try_clone() {
            Ok(socket) => socket,
            Err(e) => {
                eprintln!("{e:?}");
                return false;
            }
        };
        let mut reader = BufReader::new(client_socket);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{e:?}");
                    break;
                }
            }

            let line = line.trim_end_matches(['\r', '\n']);
            println!("Received from client: {line}");
            let parts: Vec<&str> = line.split(' ').collect();
            let command = parts[0].to_uppercase();

            match command.as_str() {
                "CREATE" => {
                    let Some(file_name) = parts.get(1) else {
                        eprintln!(
                            "IndexOutOfBoundsException: Index 1 out of bounds for length {}",
                            parts.len()
                        );
                        break;
                    };

                    match self.fs_manager.create_file(file_name) {
                        Ok(()) => {
                            if let Err(e) = writeln!(writer, "SUCCESS: File '{file_name}' created.")
                            {
                                eprintln!("{e:?}");
                                break;
                            }
                            if let Err(e) = writer.flush() {
                                eprintln!("{e:?}");
                                break;
                            }
                        }
                        Err(e) => {
                            // The Java starter code lets this exception close the connection.
                            eprintln!("{e}");
                            break;
                        }
                    }
                }
                // TODO: Implement other commands READ, WRITE, DELETE, LIST
                "QUIT" => {
                    if let Err(e) = writeln!(writer, "SUCCESS: Disconnecting.") {
                        eprintln!("{e:?}");
                    }
                    return true;
                }
                _ => {
                    if let Err(e) = writeln!(writer, "ERROR: Unknown command.") {
                        eprintln!("{e:?}");
                        break;
                    }
                    if let Err(e) = writer.flush() {
                        eprintln!("{e:?}");
                        break;
                    }
                }
            }
        }

        // Dropping reader and writer closes the client socket, like the finally block.
        false
    }
}

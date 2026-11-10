# COEN 346 – File Sharing Server (Rust translation)

This is a direct Rust translation of the Java starter code. It keeps the same
behaviour and the same `TODO` sections as the original; no assignment features
have been added.

## Layout

- `file-server/` – the server, translated from `FileServer/`
- `file-client/` – the console client and stress test, translated from `FileClient/`

## Requirements

- Rust 1.75 or later (edition 2021)

## Running

From this `rust/` directory:

```sh
# Start the server (listens on port 12345)
cargo run --bin file-server

# In another terminal, start the console client
cargo run --bin file-client

# Run the stress test (same arguments/defaults as the Java version)
cargo run --bin stress-test-client
cargo run --bin stress-test-client -- 200 10 0.3
```

The server currently implements only what the Java starter server implemented:
`CREATE` (whose file-system method is still a `TODO`), `QUIT`, and the unknown
command reply. `READ`, `WRITE`, `DELETE`, and `LIST` are left as `TODO`s, just
as in the starter code.

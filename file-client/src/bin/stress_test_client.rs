use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

/**
 * Stress test for the file sharing server.
 *
 * Starts many clients at the same moment. Every client opens its own connection and
 * sends a mix of READ and WRITE requests on one shared file, plus a LIST and two
 * malformed requests. The test then reports how many requests got a valid answer,
 * how long the clients waited, and whether any client ever read a file that was
 * only partially written (a sign of missing synchronization).
 *
 * Each client pauses briefly between requests, as a person would, so it keeps its
 * connection open for a while. A server that serves one client at a time makes the
 * other clients wait, and many of them will time out.
 *
 * Usage (all arguments are optional):
 *   stress-test-client [clients] [rounds] [writeRatio] [pauseMs] [host] [port]
 *   default:             100       10       0.2          50        localhost 12345
 *
 * The test needs CREATE, WRITE, READ and LIST to be implemented. A correct
 * multithreaded server answers every request, with no torn reads.
 */

// Content written to the shared file: one repeated letter, longer than one block
// (128 bytes), so that a write always spans more than one data block.
const CONTENT_LENGTH: usize = 200;
const SHARED_FILE: &str = "shared";
const TIMEOUT_MS: u64 = 10_000;

static REQUESTS: AtomicUsize = AtomicUsize::new(0);
static VALID_REPLIES: AtomicUsize = AtomicUsize::new(0);
static EXPECTED_ERRORS: AtomicUsize = AtomicUsize::new(0);
static UNEXPECTED_ERRORS: AtomicUsize = AtomicUsize::new(0);
static TORN_READS: AtomicUsize = AtomicUsize::new(0);
static UNEXPECTED_REPLIES: AtomicUsize = AtomicUsize::new(0);
static TIMEOUTS: AtomicUsize = AtomicUsize::new(0);
static CONNECTION_FAILURES: AtomicUsize = AtomicUsize::new(0);
static COMPLETED_CLIENTS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_LATENCY_NANOS: AtomicU64 = AtomicU64::new(0);
static MAX_LATENCY_NANOS: AtomicU64 = AtomicU64::new(0);

enum ClientFailure {
    Timeout,
    Connection,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let clients: usize = args
        .get(1)
        .map_or(100, |s| s.parse().expect("invalid clients"));
    let rounds: usize = args
        .get(2)
        .map_or(10, |s| s.parse().expect("invalid rounds"));
    let write_ratio: f64 = args
        .get(3)
        .map_or(0.2, |s| s.parse().expect("invalid write ratio"));
    let pause_ms: u64 = args
        .get(4)
        .map_or(50, |s| s.parse().expect("invalid pause"));
    let host: String = args
        .get(5)
        .cloned()
        .unwrap_or_else(|| "localhost".to_string());
    let port: u16 = args
        .get(6)
        .map_or(12345, |s| s.parse().expect("invalid port"));

    println!(
        "Stress test: {} clients x {} requests each ({:.0}% writes, {} ms pause) on {}:{}",
        clients,
        rounds,
        write_ratio * 100.0,
        pause_ms,
        host,
        port
    );

    // Set up the shared file with a single connection before the test starts.
    if !setup(&host, port) {
        return;
    }

    let start = Arc::new(Barrier::new(clients + 1));
    let mut threads = Vec::new();
    for id in 0..clients {
        let start = Arc::clone(&start);
        let host = host.clone();
        threads.push(thread::spawn(move || {
            run_client(id, rounds, write_ratio, pause_ms, &host, port, start);
        }));
    }

    let begin = Instant::now();
    start.wait(); // all clients start at the same moment
    for thread in threads {
        let _ = thread.join();
    }
    let seconds = begin.elapsed().as_secs_f64();

    let answered = VALID_REPLIES.load(Ordering::SeqCst)
        + EXPECTED_ERRORS.load(Ordering::SeqCst)
        + UNEXPECTED_ERRORS.load(Ordering::SeqCst)
        + UNEXPECTED_REPLIES.load(Ordering::SeqCst)
        + TORN_READS.load(Ordering::SeqCst);
    println!();
    println!("=== Results ===");
    println!("Total time:              {:.2} s", seconds);
    println!(
        "Clients completed:       {} / {}",
        COMPLETED_CLIENTS.load(Ordering::SeqCst),
        clients
    );
    println!(
        "Requests sent:           {}",
        REQUESTS.load(Ordering::SeqCst)
    );
    println!("Replies received:        {}", answered);
    println!(
        "  correct replies:       {}",
        VALID_REPLIES.load(Ordering::SeqCst)
    );
    println!(
        "  ERROR to bad requests: {} (expected: 2 per client)",
        EXPECTED_ERRORS.load(Ordering::SeqCst)
    );
    println!(
        "  ERROR to good requests: {}",
        UNEXPECTED_ERRORS.load(Ordering::SeqCst)
    );
    let average_latency = if answered == 0 {
        0.0
    } else {
        TOTAL_LATENCY_NANOS.load(Ordering::SeqCst) as f64 / 1e6 / answered as f64
    };
    let max_latency = MAX_LATENCY_NANOS.load(Ordering::SeqCst) as f64 / 1e6;
    println!(
        "Average / max latency:   {:.1} ms / {:.1} ms",
        average_latency, max_latency
    );
    println!(
        "Timeouts (no reply {}s): {}",
        TIMEOUT_MS / 1000,
        TIMEOUTS.load(Ordering::SeqCst)
    );
    println!(
        "Connection failures:     {}",
        CONNECTION_FAILURES.load(Ordering::SeqCst)
    );
    println!(
        "Unexpected replies:      {}",
        UNEXPECTED_REPLIES.load(Ordering::SeqCst)
    );
    println!(
        "Torn reads:              {}",
        TORN_READS.load(Ordering::SeqCst)
    );
    println!();
    if COMPLETED_CLIENTS.load(Ordering::SeqCst) == clients
        && TIMEOUTS.load(Ordering::SeqCst) == 0
        && CONNECTION_FAILURES.load(Ordering::SeqCst) == 0
        && TORN_READS.load(Ordering::SeqCst) == 0
        && UNEXPECTED_REPLIES.load(Ordering::SeqCst) == 0
        && UNEXPECTED_ERRORS.load(Ordering::SeqCst) == 0
        && EXPECTED_ERRORS.load(Ordering::SeqCst) == 2 * clients
    {
        println!("PASS: every client was served and every read was consistent.");
    } else {
        println!("FAIL: see the counts above.");
    }
}

fn run_client(
    id: usize,
    rounds: usize,
    write_ratio: f64,
    pause_ms: u64,
    host: &str,
    port: u16,
    start: Arc<Barrier>,
) {
    let mut random = JavaRandom::new(id as i64);
    start.wait();

    match run_client_inner(id, rounds, write_ratio, pause_ms, host, port, &mut random) {
        Ok(()) => {
            COMPLETED_CLIENTS.fetch_add(1, Ordering::SeqCst);
        }
        Err(ClientFailure::Timeout) => {
            TIMEOUTS.fetch_add(1, Ordering::SeqCst);
        }
        Err(ClientFailure::Connection) => {
            CONNECTION_FAILURES.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn run_client_inner(
    _id: usize,
    rounds: usize,
    write_ratio: f64,
    pause_ms: u64,
    host: &str,
    port: u16,
    random: &mut JavaRandom,
) -> Result<(), ClientFailure> {
    let socket = TcpStream::connect((host, port)).map_err(|_| ClientFailure::Connection)?;
    socket
        .set_read_timeout(Some(Duration::from_millis(TIMEOUT_MS)))
        .map_err(|_| ClientFailure::Connection)?;

    let mut writer = socket.try_clone().map_err(|_| ClientFailure::Connection)?;
    let mut reader = BufReader::new(socket);

    // Malformed requests: the server must answer with an ERROR and keep the connection open.
    expect_error(&mut writer, &mut reader, "HELLO")?;
    expect_error(&mut writer, &mut reader, "READ")?;

    for _ in 0..rounds {
        thread::sleep(Duration::from_millis(pause_ms));
        if random.next_double() < write_ratio {
            let letter = (b'A' + random.next_int(26) as u8) as char;
            let content: String = std::iter::repeat(letter).take(CONTENT_LENGTH).collect();
            let reply = send(
                &mut writer,
                &mut reader,
                &format!("WRITE {SHARED_FILE} {content}"),
            )?;
            classify(&reply, "SUCCESS");
        } else {
            let reply = send(&mut writer, &mut reader, &format!("READ {SHARED_FILE}"))?;
            check_read(&reply);
        }
    }

    let list = send(&mut writer, &mut reader, "LIST")?;
    if list.starts_with("ERROR") {
        UNEXPECTED_ERRORS.fetch_add(1, Ordering::SeqCst);
    } else if list.contains(SHARED_FILE) {
        VALID_REPLIES.fetch_add(1, Ordering::SeqCst);
    } else {
        UNEXPECTED_REPLIES.fetch_add(1, Ordering::SeqCst);
    }

    Ok(())
}

/// Sends one request and waits for the one-line reply. Returns an error on timeout or closed connection.
fn send(
    out: &mut TcpStream,
    input: &mut BufReader<TcpStream>,
    request: &str,
) -> Result<String, ClientFailure> {
    REQUESTS.fetch_add(1, Ordering::SeqCst);
    let t0 = Instant::now();
    writeln!(out, "{request}").map_err(|_| ClientFailure::Connection)?;
    out.flush().map_err(|_| ClientFailure::Connection)?;

    let mut reply = String::new();
    let read = input.read_line(&mut reply).map_err(classify_io_error)?;
    if read == 0 {
        return Err(ClientFailure::Connection);
    }

    let latency = t0.elapsed().as_nanos() as u64;
    TOTAL_LATENCY_NANOS.fetch_add(latency, Ordering::SeqCst);
    MAX_LATENCY_NANOS.fetch_max(latency, Ordering::SeqCst);
    Ok(reply.trim_end_matches(['\r', '\n']).to_string())
}

fn classify_io_error(e: io::Error) -> ClientFailure {
    match e.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => ClientFailure::Timeout,
        _ => ClientFailure::Connection,
    }
}

fn expect_error(
    out: &mut TcpStream,
    input: &mut BufReader<TcpStream>,
    request: &str,
) -> Result<(), ClientFailure> {
    let reply = send(out, input, request)?;
    if reply.starts_with("ERROR") {
        EXPECTED_ERRORS.fetch_add(1, Ordering::SeqCst);
    } else {
        UNEXPECTED_REPLIES.fetch_add(1, Ordering::SeqCst);
    }
    Ok(())
}

fn classify(reply: &str, expected_prefix: &str) {
    if reply.starts_with(expected_prefix) {
        VALID_REPLIES.fetch_add(1, Ordering::SeqCst);
    } else if reply.starts_with("ERROR") {
        UNEXPECTED_ERRORS.fetch_add(1, Ordering::SeqCst);
    } else {
        UNEXPECTED_REPLIES.fetch_add(1, Ordering::SeqCst);
    }
}

/// A consistent read is CONTENT_LENGTH copies of the same letter. Anything else is a torn read.
fn check_read(reply: &str) {
    if reply.starts_with("ERROR") {
        UNEXPECTED_ERRORS.fetch_add(1, Ordering::SeqCst);
        return;
    }

    let bytes = reply.as_bytes();
    let uniform = bytes.len() == CONTENT_LENGTH && bytes.iter().all(|&b| b == bytes[0]);
    if uniform {
        VALID_REPLIES.fetch_add(1, Ordering::SeqCst);
    } else {
        TORN_READS.fetch_add(1, Ordering::SeqCst);
    }
}

fn setup(host: &str, port: u16) -> bool {
    match setup_inner(host, port) {
        Ok(success) => success,
        Err(e) => {
            println!("Setup failed ({e}). Is the server running?");
            false
        }
    }
}

fn setup_inner(host: &str, port: u16) -> Result<bool, String> {
    let socket = TcpStream::connect((host, port)).map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_millis(TIMEOUT_MS)))
        .map_err(|e| e.to_string())?;

    let mut writer = socket.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(socket);

    let created = setup_send(&mut writer, &mut reader, &format!("CREATE {SHARED_FILE}"))?;
    println!("Setup  CREATE {SHARED_FILE} -> {created}");

    let content = "A".repeat(CONTENT_LENGTH);
    let written = setup_send(
        &mut writer,
        &mut reader,
        &format!("WRITE {SHARED_FILE} {content}"),
    )?;
    println!("Setup  WRITE  {SHARED_FILE} -> {written}");

    if !written.starts_with("SUCCESS") {
        println!(
            "Setup failed: the server must implement CREATE and WRITE before this test can run."
        );
        println!(
            "(If '{SHARED_FILE}' already exists from a previous run, restart the server with a fresh file system.)"
        );
        return Ok(false);
    }

    Ok(true)
}

fn setup_send(
    out: &mut TcpStream,
    input: &mut BufReader<TcpStream>,
    request: &str,
) -> Result<String, String> {
    writeln!(out, "{request}").map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;

    let mut reply = String::new();
    let read = input.read_line(&mut reply).map_err(|e| e.to_string())?;
    if read == 0 {
        return Err("server closed the connection".to_string());
    }
    Ok(reply.trim_end_matches(['\r', '\n']).to_string())
}

/// A small copy of java.util.Random so the test produces the same request mix as the Java version.
struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    const MULTIPLIER: u64 = 0x5DEECE66D;
    const ADDEND: u64 = 0xB;
    const MASK: u64 = (1 << 48) - 1;

    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND)
            & Self::MASK;
        (self.seed >> (48 - bits)) as u32
    }

    fn next_double(&mut self) -> f64 {
        let high = self.next(26) as u64;
        let low = self.next(27) as u64;
        ((high << 27) + low) as f64 / (1u64 << 53) as f64
    }

    fn next_int(&mut self, bound: u32) -> u32 {
        if bound & bound.wrapping_neg() == bound {
            return ((bound as u64 * self.next(31) as u64) >> 31) as u32;
        }

        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits - val + (bound - 1) < (1u32 << 31) {
                return val;
            }
        }
    }
}

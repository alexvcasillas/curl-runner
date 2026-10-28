//! Standalone test server: `cargo run -p testkit --bin testkit-server -- --port <p>`.
//! Prints `listening <base_url>` once ready and serves until killed.

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let port = match args.as_slice() {
        [] => 0,
        [flag, p] if flag == "--port" => p.parse::<u16>().unwrap_or_else(|_| {
            eprintln!("invalid port: {p}");
            std::process::exit(2);
        }),
        _ => {
            eprintln!("usage: testkit-server [--port <port>]");
            std::process::exit(2);
        }
    };
    let server = testkit::TestServer::start_on(port);
    println!("listening {}", server.base_url);
    let _ = std::io::stdout().flush();
    loop {
        std::thread::park();
    }
}

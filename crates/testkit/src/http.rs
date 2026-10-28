//! Minimal HTTP/1.1 server: one thread per connection, keep-alive,
//! `Content-Length` and chunked request bodies, `Expect: 100-continue`.
//!
//! Replaces `tiny_http` 0.12, whose 4-thread connection pool could lose the
//! wake-up for a burst of new connections: with more than 4 idle keep-alive
//! connections held open (libcurl's connection cache in a parallel run), the
//! extra requests were never answered and the client waited forever.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Idle keep-alive connections are closed after this long without a request.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
/// Value of the `Server` header (kept from tiny_http so outputs stay stable).
const SERVER_NAME: &str = "tiny-http (Rust)";

/// One parsed request (body fully read).
pub struct Request {
    pub method: String,
    pub url: String,
    /// Header name (as sent) and value, in order.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn from_data(body: Vec<u8>) -> Response {
        Response {
            status: 200,
            headers: Vec::new(),
            body,
        }
    }

    pub fn with_status_code(mut self, status: u16) -> Response {
        self.status = status;
        self
    }

    /// Adds a header; a second `Content-Type` replaces the first.
    pub fn with_header(mut self, name: &str, value: &str) -> Response {
        if name.eq_ignore_ascii_case("content-type") {
            self.headers
                .retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
        }
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

/// A listening server; the accept thread stops on [`Listener::stop`].
pub struct Listener {
    pub port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Listener {
    pub fn start<H>(port: u16, handler: H) -> Listener
    where
        H: Fn(Request) -> Response + Send + Sync + 'static,
    {
        let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind test server");
        let port = listener.local_addr().expect("local addr").port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let handler = Arc::new(handler);
        let thread = thread::spawn(move || {
            for conn in listener.incoming() {
                if stop2.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(conn) = conn else { continue };
                let handler = handler.clone();
                thread::spawn(move || {
                    let _ = serve(conn, &*handler);
                });
            }
        });
        Listener {
            port,
            stop,
            thread: Some(thread),
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Wake the blocking accept.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Serve requests on one connection until the client closes it, asks to
/// close, or stays idle past [`IDLE_TIMEOUT`].
fn serve(conn: TcpStream, handler: &dyn Fn(Request) -> Response) -> io::Result<()> {
    conn.set_read_timeout(Some(IDLE_TIMEOUT))?;
    let _ = conn.set_nodelay(true);
    let mut out = conn.try_clone()?;
    let mut rd = BufReader::new(conn);
    loop {
        let mut line = String::new();
        if rd.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let line = line.trim_end();
        if line.is_empty() {
            continue; // stray CRLF between requests
        }
        let mut parts = line.split(' ');
        let (Some(method), Some(url), Some(version)) = (parts.next(), parts.next(), parts.next())
        else {
            write_response(&mut out, "HTTP/1.1", false, &bad(400))?;
            return Ok(());
        };
        let (method, url, version) = (method.to_string(), url.to_string(), version.to_string());
        let mut headers = Vec::new();
        loop {
            let mut h = String::new();
            if rd.read_line(&mut h)? == 0 {
                return Ok(());
            }
            let h = h.trim_end();
            if h.is_empty() {
                break;
            }
            match h.split_once(':') {
                Some((k, v)) => headers.push((k.trim().to_string(), v.trim().to_string())),
                None => {
                    write_response(&mut out, &version, false, &bad(400))?;
                    return Ok(());
                }
            }
        }
        let mut req = Request {
            method,
            url,
            headers,
            body: Vec::new(),
        };
        if req
            .header("expect")
            .is_some_and(|v| v.eq_ignore_ascii_case("100-continue"))
        {
            out.write_all(format!("{version} 100 Continue\r\n\r\n").as_bytes())?;
        }
        req.body = read_body(&mut rd, &req)?;
        let conn_hdr = req.header("connection").map(str::to_ascii_lowercase);
        let keep_alive = match conn_hdr.as_deref() {
            Some(v) if v.contains("close") => false,
            Some(v) if v.contains("keep-alive") => true,
            _ => version != "HTTP/1.0",
        };
        let head = req.method == "HEAD";
        let resp = handler(req);
        write_response(&mut out, &version, head, &resp)?;
        if !keep_alive {
            return Ok(());
        }
    }
}

fn bad(status: u16) -> Response {
    Response::from_data(Vec::new()).with_status_code(status)
}

fn read_body(rd: &mut BufReader<TcpStream>, req: &Request) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    if req
        .header("transfer-encoding")
        .is_some_and(|v| v.to_ascii_lowercase().contains("chunked"))
    {
        loop {
            let mut size = String::new();
            rd.read_line(&mut size)?;
            let size = size.trim().split(';').next().unwrap_or("");
            let n = usize::from_str_radix(size, 16)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad chunk size"))?;
            if n == 0 {
                // trailers until the empty line
                loop {
                    let mut t = String::new();
                    if rd.read_line(&mut t)? == 0 || t.trim_end().is_empty() {
                        return Ok(body);
                    }
                }
            }
            let start = body.len();
            body.resize(start + n, 0);
            rd.read_exact(&mut body[start..])?;
            let mut crlf = [0u8; 2];
            rd.read_exact(&mut crlf)?;
        }
    }
    if let Some(n) = req
        .header("content-length")
        .and_then(|v| v.parse::<usize>().ok())
    {
        body.resize(n, 0);
        rd.read_exact(&mut body)?;
    }
    Ok(body)
}

fn write_response(out: &mut TcpStream, version: &str, head: bool, r: &Response) -> io::Result<()> {
    let version = if version == "HTTP/1.0" {
        "HTTP/1.0"
    } else {
        "HTTP/1.1"
    };
    let mut s = format!(
        "{version} {} {}\r\nServer: {SERVER_NAME}\r\nDate: {}\r\n",
        r.status,
        reason(r.status),
        http_date(SystemTime::now())
    );
    for (k, v) in &r.headers {
        s.push_str(&format!("{k}: {v}\r\n"));
    }
    s.push_str(&format!("Content-Length: {}\r\n\r\n", r.body.len()));
    let mut buf = s.into_bytes();
    if !head {
        buf.extend_from_slice(&r.body);
    }
    out.write_all(&buf)?;
    out.flush()
}

fn reason(code: u16) -> &'static str {
    match code {
        100 => "Continue",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
}

/// RFC 1123 date, e.g. `Tue, 27 Oct 2026 20:37:36 GMT`.
fn http_date(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    // days since 1970-01-01 -> civil date (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(mo <= 2);
    const WD: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MO: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{}, {d:02} {} {y} {h:02}:{m:02}:{s:02} GMT",
        WD[days.rem_euclid(7) as usize],
        MO[(mo - 1) as usize]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_date_formats_rfc1123() {
        // 2026-10-27T20:37:36Z
        let t = UNIX_EPOCH + Duration::from_secs(1_793_133_456);
        assert_eq!(http_date(t), "Tue, 27 Oct 2026 20:37:36 GMT");
        assert_eq!(http_date(UNIX_EPOCH), "Thu, 01 Jan 1970 00:00:00 GMT");
    }
}

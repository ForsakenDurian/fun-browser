use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use native_tls::TlsConnector;

const USER_AGENT: &str = "goated-browser";
const VERSION: &str = env!("CARGO_PKG_VERSION");

// Anything we can send a request over: a plain TcpStream or a TLS-wrapped one.
trait Stream: Read + Write {}
impl<T: Read + Write> Stream for T {}

pub struct URL {
    scheme: String,
    host: String,
    port: u16,
    path: String,
}

impl URL {
    pub fn new(url: &str) -> Self {
        let (scheme, rest) = url.split_once("://").expect("URL has no scheme");

        let mut rest = rest.to_string();

        if !rest.contains('/') {
            rest = rest + "/";
        }
        let (host, path) = rest.split_once('/').unwrap();
        let path = format!("/{path}");

        let port: u16 = match scheme {
            "http" => 80,
            "https" => 443,
            "file" => 0,
            _ => panic!("Unsupported scheme {}", scheme),
        };
        let (host, port) = match host.split_once(':') {
            Some((h,p)) => (h, p.parse::<u16>().expect("bad port")),
            None => (host, port),
        };

        URL {
            scheme: scheme.to_string(),
            host: host.to_string(),
            port,
            path,
        }
    }

    pub fn request(&self) -> String {
        if self.scheme == "file" {
            return std::fs::read_to_string(&self.path).expect("could not read file");
        }
        let tcp = TcpStream::connect((self.host.as_str(), self.port))
            .expect("Unable to connect to server");
        let mut stream: Box<dyn Stream> = if self.scheme == "https" {
            let connector = TlsConnector::new().expect("TLS setup failed");
            Box::new(
                connector
                    .connect(&self.host, tcp)
                    .expect("TLS handshake failed"),
            )
        } else {
            Box::new(tcp)
        };

        let mut request = format!("GET {} HTTP/1.0\r\n", self.path);
        request.push_str(&format!("Host: {}\r\n", self.host));
        request.push_str(&format!("User-Agent: {}/{}\r\n", USER_AGENT, VERSION));
        request.push_str("\r\n");
        stream.write_all(&request.as_bytes()).expect("write failed");

        let mut response = BufReader::new(stream);

        let mut status_line = String::new();
        response.read_line(&mut status_line).expect("read failed");
        let (version, rest) = status_line.split_once(' ').expect("bad status line");
        let (status, explanation) = rest.split_once(' ').expect("bad status line");

        let mut response_headers: HashMap<String, String> = HashMap::new();
        for line in response.by_ref().lines() {
            let line = line.expect("read failed");
            if line.is_empty() {
                break;
            }
            let (header, value) = line.split_once(':').expect("bad header line");
            response_headers.insert(header.to_ascii_lowercase(), value.trim().to_string());
        }
        assert!(!response_headers.contains_key("transfer-encoding"));
        assert!(!response_headers.contains_key("content-encoding"));

        let mut content = String::new();
        response.read_to_string(&mut content).expect("read failed");
        content
    }
}

pub fn show(body: &str) {
    let mut in_tag = false;
    for c in body.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            print!("{c}");
        }
    }
}

pub fn load(url: URL) {
    let body = url.request();
    show(&body);
}
fn main() {
    let url = std::env::args().nth(1).unwrap_or_else(|| format!("file://{}/test.html", env!("CARGO_MANIFEST_DIR")));
    load(URL::new(&url));
}

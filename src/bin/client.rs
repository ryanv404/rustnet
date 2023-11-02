use std::env;
use std::io;

use rustnet::Client;

// Simple coloring on unix-like systems.
#[cfg(unix)]
pub mod ansi {
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const CYAN: &str = "\x1b[36m";
    pub const RESET: &str = "\x1b[0m";
}

#[cfg(unix)]
use ansi::*;

// Skip coloring if not on a unix-like system.
#[cfg(not(unix))]
pub mod not_ansi {
    pub const RED: &str = "";
    pub const GREEN: &str = "";
    pub const CYAN: &str = "";
    pub const RESET: &str = "";
}

#[cfg(not(unix))]
use not_ansi::*;

const HELP_MSG: &str = "\
Usage:\n  \
  client <addr> [uri] [data]\n\n\
Arguments:\n  \
  addr   A remote host's URL or IP address.\n  \
  uri    Target resource on the remote host (default: \"/\").\n  \
  data   Data to be sent in the request body (optional).\n\n\
Options:\n  \
  -h, --help  Displays this help message.\
";

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);

    if let Some(addr) = args.next() {
        if addr.eq_ignore_ascii_case("--help") ||
           addr.eq_ignore_ascii_case("-h")
        {
            println!("{HELP_MSG}");
            return Ok(());
        }

        let addr = format!("{addr}:80");
        let uri = args.next().unwrap_or_else(|| String::from("/"));
        let _data = args.next().unwrap_or_default();

        // Create an HTTP client and send a request.
        let mut client = Client::http()
            .addr(&addr)
            .uri(&uri)
            .send()?;

        println!("{CYAN}---[Request]--->{RESET}\n{}\n", &client);

        let res = client.recv()?;

        println!("{GREEN}<---[Response]---{RESET}\n{res}");

        Ok(())
    } else {
        eprintln!("{RED}Must provide a URL or IP address.{RESET}\n\n{HELP_MSG}");
        Ok(())
    }
}

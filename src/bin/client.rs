use std::{env, io};

use rustnet::Client;

pub mod ansi {
    pub const RED: &str = "\x1b[91m";
    pub const GREEN: &str = "\x1b[92m";
    pub const PURP: &str = "\x1b[95m";
    pub const CYAN: &str = "\x1b[96m";
    pub const RESET: &str = "\x1b[0m";
}

use ansi::*;

const HELP_MSG: &str = "\
Usage:\n  \
  client <addr> [uri] [data]\n\n\
Arguments:\n  \
  addr   A remote host's URL or IP address.\n  \
  uri    Target URI on the remote host (default: \"/\").\n  \
  data   Data to be sent in the request body (optional).\n\n\
Options:\n  \
  -h, --help  Displays this help message.\
";

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);

    let Some(addr) = args.next() else {
        eprintln!("{RED}Must provide a URL or IP address.{RESET}\n\n{HELP_MSG}");
        return Ok(());
    };

    if addr.eq_ignore_ascii_case("--help") || addr.eq_ignore_ascii_case("-h") {
        println!("{HELP_MSG}");
        return Ok(());
    }

    let addr = format!("{addr}:80");
    let uri = args.next().unwrap_or_else(|| String::from("/"));
    //let data = args.next().unwrap_or_default();

    // Create an HTTP client and send a request.
    let mut client = Client::http()
        .addr(&addr)
        .uri(&uri)
        .send()?;

    println!("{PURP}---[Request]--->\n{}{RESET}\n", &client);

    // Receive the response from the server.
    let res = client.recv()?;

    println!("{CYAN}<---[Response]---\n{res}{RESET}");

    Ok(())
}

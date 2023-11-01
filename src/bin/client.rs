use std::{env, io};

use rustnet::Client;

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

        println!("--[Request]-->\n{}\n", &client);

        let res = client.recv()?;

        println!("<--[Response]--\n{res}");

        Ok(())
    } else {
        eprintln!("Must provide a URL or IP address.\n\n{HELP_MSG}");
        Ok(())
    }
}

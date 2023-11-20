use std::{env, io};

use rustnet::{Client, consts::DATE};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const YLW: &str = "\x1b[93m";
const CYAN: &str = "\x1b[96m";
const CLR: &str = "\x1b[0m";

#[rustfmt::skip]
fn main() -> io::Result<()> {
    let mut is_testing = false;
    let mut args = env::args().skip(1);

    // Process CLI arguments until first non-option argument.
    let addr = loop {
        match args.next() {
            // End of options.
            Some(opt) if opt == "--" => {
                if let Some(addr) = args.next() {
                    // First non-option argument is the addr argument.
                    break addr;
                } else {
                    // Unexpected end of arguments.
                    eprintln!("{RED}Missing a hostname or IP:port address.{CLR}\n");
                    show_help();
                    return Ok(());
                }
            },
            // Handle an option.
            Some(opt) if opt.starts_with('-') => match &*opt {
                "-h" | "--help" => {
                    show_help();
                    return Ok(());
                },
                "--testing" => is_testing = true,
                _ => {
                    // Unknown option.
                    eprintln!("{RED}Unknown option: {opt}{CLR}\n");
                    show_help();
                    return Ok(());
                },
            },
            // First non-option argument is the addr argument.
            Some(addr) => break addr,
            // Unexpected end of arguments.
            None => {
                eprintln!("{RED}Missing a hostname or IP:port address.{CLR}\n");
                show_help();
                return Ok(());
            },
        }
    };

    // Process the remainder of the arguments or use default values.
    let addr = if is_testing {
        addr
    } else {
        format!("{addr}:80")
    };

    let path = args.next().unwrap_or_else(|| String::from("/"));
    let body = args.next().unwrap_or_default();

    // Create an HTTP client and send a request.
    let mut client = Client::http()
        .addr(&addr)
        .path(&path)
        .body(body.as_bytes())
        .send()?;

    // Receive the response from the server.
    let mut res = client.recv()?;

    if is_testing {
		// Ignore Date headers in tests.
        client.req.headers.remove(&DATE);
        res.headers.remove(&DATE);
    }

    if is_testing {
		let req_line = client.request_line();
		let status_line = res.status_line();

		let req_headers = client.headers_to_string();
		let res_headers = res.headers_to_string();

		let request = format!("{req_line}\n{}", req_headers.trim_end());
		let response = format!("{status_line}\n{}", res_headers.trim_end());
		println!("{request}\n\n{response}");
    } else {
		let request = client.to_string();
		let request = request.trim_end();

		let response = res.to_string();
		let response = response.trim_end();

		println!("{YLW}--[Request]-->\n{request}{CLR}\n");
        println!("{CYAN}<--[Response]--\n{response}{CLR}");
    }

    Ok(())
}

fn show_help() {
    let help_msg = format!("\
        {GRN}USAGE{CLR}\n    \
            client <addr> [path] [body]\n\n\
        {GRN}ARGUMENTS{CLR}\n    \
            addr    A hostname or IP:port address (e.g. \"httpbin.org\").\n    \
            path    URI path component on the target resource (default: \"/\").\n    \
            body    Data to be sent in the request body (optional).\n\n\
        {GRN}OPTIONS{CLR}\n    \
            -h, --help    Displays this help message.\n    \
            --testing     The Date header is stripped and output is not colorized.\
    ");

    eprintln!("{help_msg}");
}

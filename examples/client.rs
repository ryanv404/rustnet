use std::{env, io};

use rustnet::{Client, consts::DATE};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const YLW: &str = "\x1b[93m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

#[rustfmt::skip]
fn main() -> io::Result<()> {
    let mut is_testing = false;
    let mut args = env::args().skip(1);

    // Process CLI arguments until first non-option argument.
    let (addr, path, body) = loop {
        match args.next() {
            // End of options.
            Some(opt) if opt == "--" => {
                if let Some(uri) = args.next() {
                    // First non-option argument is the URI argument.
                    if let Some((addr, path)) = Client::parse_uri(&uri) {
                        let body = args.next().unwrap_or_default();
                        break (addr, path, body);
                    } else {
                        eprintln!("{RED}Unable to parse the URI argument.{CLR}\n");
                        return Ok(());
                    }
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
            // First non-option argument is the URI argument.
            Some(uri) => {
                if let Some((addr, path)) = Client::parse_uri(&uri) {
                    let body = args.next().unwrap_or_default();
                    break (addr, path, body);
                } else {
                    eprintln!("{RED}Unable to parse the URI argument.{CLR}\n");
                    return Ok(());
                }
            },
            // Unexpected end of arguments.
            None => {
                eprintln!("{RED}Missing URI argument.{CLR}\n");
                show_help();
                return Ok(());
            },
        }
    };

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
		println!(
		    "{}\n{}\n\n{}\n{}",
		    client.request_line(),
            client.headers_to_string().trim_end(),
		    res.status_line(),
            res.headers_to_string().trim_end()
	    );
    } else {
        let req_body = client.req.body_to_string();
        let res_body = res.body_to_string();
        let res_color = if res.status_code() >= 400 {
            PURP
        } else {
            GRN
        };

        match (req_body.len(), res_body.len()) {
            (0, 0) => {
        		println!(
        		    "{YLW}{}{CLR}\n{}\n\n{res_color}{}{CLR}\n{}\n",
        		    client.request_line(),
                    client.headers_to_string().trim_end(),
        		    res.status_line(),
                    res.headers_to_string().trim_end()
        	    );
            },
            (_, 0) => {
        		println!(
        		    "{YLW}{}{CLR}\n{}\n{}\n\n{res_color}{}{CLR}\n{}\n",
        		    client.request_line(),
                    client.headers_to_string().trim_end(),
                    req_body.trim_end(),
        		    res.status_line(),
                    res.headers_to_string().trim_end()
        	    );
            },
            (0, _) => {
        		println!(
        		    "{YLW}{}{CLR}\n{}\n\n{res_color}{}{CLR}\n{}\n\n{}\n",
        		    client.request_line(),
                    client.headers_to_string().trim_end(),
        		    res.status_line(),
                    res.headers_to_string().trim_end(),
                    res_body.trim_end()
        	    );
            },
            (_, _) => {
        		println!(
        		    "{YLW}{}{CLR}\n{}\n{}\n\n{res_color}{}{CLR}\n{}\n\n{}\n",
        		    client.request_line(),
                    client.headers_to_string().trim_end(),
                    req_body.trim_end(),
        		    res.status_line(),
                    res.headers_to_string().trim_end(),
                    res_body.trim_end()
        	    );
            },
        }
    }

    Ok(())
}

fn show_help() {
    eprintln!("\
        {GRN}USAGE{CLR}\n    \
            client <uri> [body]\n\n\
        {GRN}ARGUMENTS{CLR}\n    \
            uri    An HTTP URI to a remote host (e.g. \"httpbin.org/json\").\n    \
            body   Data to be sent in the request body (optional).\n\n\
        {GRN}OPTIONS{CLR}\n    \
            -h, --help    Displays this help message.\n    \
            --testing     The Date header is stripped and output is not colorized.\
    ");
}

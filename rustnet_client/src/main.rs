use std::{env, io};

use librustnet::{Client, consts::DATE, Method};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

#[rustfmt::skip]
fn main() -> io::Result<()> {
    let mut path_arg = None;
    let mut method_arg = None;
    let mut testing_client = false;
    let mut testing_server = false;
    let mut args = env::args().skip(1);

    // Handle the CLI options.
    let uri = loop {
        match args.next() {
            // End of options flag.
            Some(opt) if opt == "--" => {
                if let Some(uri) = args.next() {
                    // First non-option argument should be the URI.
                    break uri;
                } else {
                    // Unexpected end of arguments.
                    eprintln!("{RED}URI argument is missing.{CLR}\n");
                    show_help();
                    return Ok(());
                }
            },
            // Options start with a "--".
            Some(opt) if opt.starts_with("--") => match &*opt {
                // Print help message.
                "--help" => {
                    show_help();
                    return Ok(());
                },
                // Uri path.
                "--path" => {
                    if let Some(uri_path) = args.next() {
                        path_arg = Some(uri_path);
                    } else {
                        // Missing custom method argument.
                        eprintln!("{RED}Missing required argument to `--path` option.{CLR}\n");
                        return Ok(());
                    }
                },
                // Custom method.
                "--method" => {
                    let maybe_method = args
                        .next()
                        .as_ref()
                        .and_then(|m| {
                            let upper = m.to_ascii_uppercase();
                            upper.parse::<Method>().ok()
                        });

                    if let Some(new_method) = maybe_method {
                        method_arg = Some(new_method);
                    } else {
                        // Missing custom method argument.
                        eprintln!("{RED}Missing required argument to `--missing` option.{CLR}\n");
                        return Ok(());
                    }
                },
                // Set the client testing output style.
                "--client-tests" => testing_client = true,
                // Set the server testing output style.
                "--server-tests" => testing_server = true,
                // Unknown option.
                _ => {
                    eprintln!("{RED}Unknown option: {opt}{CLR}\n");
                    show_help();
                    return Ok(());
                },
            },
            // First non-option argument should be the URI argument.
            Some(uri) => break uri,
            // Unexpected end of arguments.
            None => {
                eprintln!("{RED}URI argument is missing.{CLR}\n");
                show_help();
                return Ok(());
            },
        }
    };

    // Parse the URI argument.
    let (ref addr, ref path, ref body) = match Client::parse_uri(&uri) {
        Ok((addr, path)) => {
            let body = args.next().unwrap_or_default();
            (addr, path_arg.unwrap_or(path), body)
        },
        Err(_) => {
            eprintln!("{RED}Unable to parse the URI argument.{CLR}\n");
            return Ok(());
        },
    };

    // Create an HTTP client and send a request.
    let mut client = Client::http()
        .method(method_arg.unwrap_or(Method::Get))
        .addr(addr)
        .path(path)
        .body(body.as_bytes())
        .send()?;

    // Receive the response from the server.
    let mut res = client.recv()?;

    // Ignore Date headers in tests.
    if testing_client || testing_server {
        if let Some(headers) = client.req.headers.as_mut() {
            headers.remove(&DATE);
            res.headers.remove(&DATE);
        }
    }

    if testing_client {
		println!(
            "{}\n{}\n\n{}\n{}",
            client.request_line(),
            client.headers_to_string().trim_end(),
            res.status_line(),
            res.headers_to_string().trim_end()
        );
    } else if testing_server {
        let res_body = res.body_to_string();
        let res_body = res_body.trim_end();
        if res_body.is_empty() {
            println!(
                "{}\n{}",
                res.status_line(),
                res.headers_to_string().trim_end()
            );
        } else {
            println!(
                "{}\n{}\n\n{}",
                res.status_line(),
                res.headers_to_string().trim_end(),
                res_body.trim_end()
            );
        }
    } else {
        let req_body = client.req.body_to_string();
        let res_body = res.body_to_string();
        let res_color = if res.status_code() >= 400 {
            RED
        } else {
            GRN
        };

        match (req_body.len(), res_body.len()) {
            (0, 0) => {
                println!(
                    "{PURP}{}{CLR}\n{}\n\n{res_color}{}{CLR}\n{}\n",
                    client.request_line(),
                    client.headers_to_string().trim_end(),
                    res.status_line(),
                    res.headers_to_string().trim_end()
                );
            },
            (_, 0) => {
                println!(
                    "{PURP}{}{CLR}\n{}\n{}\n\n{res_color}{}{CLR}\n{}\n",
                    client.request_line(),
                    client.headers_to_string().trim_end(),
                    req_body.trim_end(),
                    res.status_line(),
                    res.headers_to_string().trim_end()
                );
            },
            (0, _) => {
                println!(
                    "{PURP}{}{CLR}\n{}\n\n{res_color}{}{CLR}\n{}\n\n{}\n",
                    client.request_line(),
                    client.headers_to_string().trim_end(),
                    res.status_line(),
                    res.headers_to_string().trim_end(),
                    res_body.trim_end()
                );
            },
            (_, _) => {
                println!(
                    "{PURP}{}{CLR}\n{}\n{}\n\n{res_color}{}{CLR}\n{}\n\n{}\n",
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
    let name = env!("CARGO_BIN_NAME");
    eprintln!("\
        {GRN}Usage:{CLR} {name} <URI> [DATA]\n\n\
        {GRN}Arguments:{CLR}\n    \
            URI    An HTTP URI to a remote host (e.g. \"httpbin.org/json\").\n    \
            DATA   Data to be sent in the request body (optional).\n\n\
        {GRN}Options:{CLR}\n    \
            -h, --help       Displays this help message.\n    \
            --client-tests   Use output style expected by client tests.\n    \
            --method METHOD  Use METHOD as the request method (default: \"GET\").\n    \
            --path PATH      Use PATH as the URI path (default: \"/\").\n    \
            --server-tests   Use output style expected by server tests.\n\
    ");
}

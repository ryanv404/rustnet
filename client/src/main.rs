use std::{env, io};

use librustnet::{Client, Method};
use librustnet::consts::DATE;

mod tui;
#[allow(unused)]
use tui::run_tui_browser;

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
                    let opt_arg = args.next();
                    let Some(method) = opt_arg.as_ref() else {
                        // Missing custom method argument.
                        eprintln!("{RED}Missing required argument to `--method` option.{CLR}\n");
                        return Ok(());
                    };

                    let Some(new_method) = method
                        .to_ascii_uppercase()
                        .parse::<Method>()
                        .ok()
                    else {
                        // Invalid method argument.
                        eprintln!("{RED}Invalid method argument.{CLR}\n");
                        return Ok(());
                    };

                    method_arg = Some(new_method);
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
    let Ok((addr, path)) = Client::parse_uri(&uri) else {
        eprintln!("{RED}Unable to parse the URI argument.{CLR}\n");
        return Ok(());
    };

    // Create an HTTP client and send a request.
    let mut client = Client::builder()
        .method(method_arg.unwrap_or(Method::Get))
        .addr(&addr)
        .path(&path_arg.as_ref().unwrap_or(&path))
        .send_text(&args.next().unwrap_or_default())?;

    // Receive the response from the server.
    client.recv()?;

    // Ignore Date headers in tests.
    if testing_client || testing_server {
        client.req.as_mut().map(|req| req.headers.remove(&DATE));
        client.res.as_mut().map(|res| res.headers.remove(&DATE));
    }

    if testing_client {
        // Only print request line, request headers, status line, and response
        // headers during client testing.
        client.req
            .as_ref()
            .map(|req| println!("{}\n{}\n",
                req.request_line(),
                req.headers_to_string().trim_end()));
        client.res
            .as_ref()
            .map(|res| println!("{}\n{}",
                res.status_line(),
                res.headers_to_string().trim_end()));
    } else if testing_server {
        // Only print the status line, response headers, and response body
        // (if it is alphanumeric data) during server testing.
        client.res
            .as_ref()
            .map(|res| println!("{}\n{}{}",
                res.status_line(),
                res.headers_to_string().trim_end(),
                if res.body.is_alphanumeric() {
                    format!("\n\n{}", res.body)
                } else {
                    String::new()
                }
            ));
    } else {
        // Handle non-testing output.
        print_output(&mut client);
    }

    Ok(())
}

fn print_output(client: &mut Client) {
    let req = client.req.take().unwrap();
    let res = client.res.take().unwrap();

    let req_body = req.body.to_string();
    let res_body = res.body.to_string();

    let res_color = if res.status_code() >= 400 {
        RED
    } else {
        GRN
    };

    match (req_body.len(), res_body.len()) {
        (0, 0) => {
            println!(
                "{PURP}{}{CLR}\n{}\n\n{res_color}{}{CLR}\n{}\n",
                req.request_line(),
                req.headers_to_string().trim_end(),
                res.status_line(),
                res.headers_to_string().trim_end()
            );
        },
        (_, 0) => {
            println!(
                "{PURP}{}{CLR}\n{}\n{}\n\n{res_color}{}{CLR}\n{}\n",
                req.request_line(),
                req.headers_to_string().trim_end(),
                req_body.trim_end(),
                res.status_line(),
                res.headers_to_string().trim_end()
            );
        },
        (0, _) => {
            println!(
                "{PURP}{}{CLR}\n{}\n\n{res_color}{}{CLR}\n{}\n\n{}\n",
                req.request_line(),
                req.headers_to_string().trim_end(),
                res.status_line(),
                res.headers_to_string().trim_end(),
                res_body.trim_end()
            );
        },
        (_, _) => {
            println!(
                "{PURP}{}{CLR}\n{}\n{}\n\n{res_color}{}{CLR}\n{}\n\n{}\n",
                req.request_line(),
                req.headers_to_string().trim_end(),
                req_body.trim_end(),
                res.status_line(),
                res.headers_to_string().trim_end(),
                res_body.trim_end()
            );
        },
    }
}

fn show_help() {
    let name = env!("CARGO_BIN_NAME");
    eprintln!("\
        {GRN}Usage:{CLR} {name} <URI> [DATA]\n\n\
        {GRN}Arguments:{CLR}\n    \
            URI    An HTTP URI to a remote host (e.g. \"httpbin.org/json\").\n    \
            DATA   Data to be sent in the request body (optional).\n\n\
        {GRN}Options:{CLR}\n    \
            --help           Displays this help message.\n    \
            --method METHOD  Use METHOD as the request method (default: \"GET\").\n    \
            --path PATH      Use PATH as the URI path (default: \"/\").\n\n\
        {GRN}Test Options:{CLR}\n    \
            --client-tests   Use output style expected by client tests.\n    \
            --server-tests   Use output style expected by server tests.\n");
}

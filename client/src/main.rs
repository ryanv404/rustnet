use std::env;
use std::io::{Result as IoResult};

use librustnet::{Client, Method};
use librustnet::consts::DATE;

mod tui;
use tui::Browser;

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

#[rustfmt::skip]
fn main() -> IoResult<()> {
    let mut path_arg = None;
    let mut method_arg = None;

    let mut testing_client = false;
    let mut testing_server = false;

    // Handle the CLI options.
    let mut args = env::args().skip(1);

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
                // Starts the client TUI.
                "--tui" => {
                    Browser::run_client_tui()?;
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
        .path(path_arg.as_ref().unwrap_or(&path))
        .send_text(&args.next().unwrap_or_default())?;

    // Receive the response from the server.
    client.recv()?;

    // Ignore Date headers in tests.
    if testing_client || testing_server {
        if let Some(req) = client.req.as_mut() {
            req.headers.remove(&DATE);
        }

        if let Some(res) = client.res.as_mut() {
            res.headers.remove(&DATE);
        }
    }

    if testing_client {
        // Only print request line, request headers, status line, and response
        // headers during client testing.
        if let Some(req) = client.req.as_ref() {
            println!(
                "{}\n{}",
                req.request_line(),
                req.headers_to_string().trim_end());
        }

        if let Some(res) = client.res.as_ref() {
            println!(
                "\n{}\n{}",
                res.status_line(),
                res.headers_to_string().trim_end());
        }
    } else if testing_server {
        // Only print the status line, response headers, and response body
        // (if it is alphanumeric data) during server testing.
        if let Some(res) = client.res.as_ref() {
            println!(
                "{}\n{}{}",
                res.status_line(),
                res.headers_to_string().trim_end(),
                if res.body.is_alphanumeric() {
                    format!("\n\n{}", res.body)
                } else {
                    String::new()
                }
            );
        }
    } else {
        // Handle non-testing output.
        print_output(&mut client);
    }

    Ok(())
}

fn print_output(client: &mut Client) {
    let req = client.req.take().unwrap();
    let res = client.res.take().unwrap();

    let res_color = if res.status_code() >= 400 {
        RED
    } else {
        GRN
    };

    let request_line = req.request_line();
    let req_headers = req.headers_to_string();
    let req_headers = req_headers.trim_end();
    let req_body = req.body.to_string();
    let req_body = req_body.trim_end();

    let status_line = res.status_line();
    let res_headers = res.headers_to_string();
    let res_headers = res_headers.trim_end();
    let res_body = res.body.to_string();
    let res_body = res_body.trim_end();

    match (req_body.len(), res_body.len()) {
        (0, 0) => {
            println!(
                "{PURP}{request_line}{CLR}\n{req_headers}\n\n\
                {res_color}{status_line}{CLR}\n{res_headers}");
        },
        (_, 0) => {
            println!(
                "{PURP}{request_line}{CLR}\n{req_headers}\n{req_body}\n\n\
                {res_color}{status_line}{CLR}\n{res_headers}");
        },
        (0, _) => {
            println!(
                "{PURP}{request_line}{CLR}\n{req_headers}\n\n\
                {res_color}{status_line}{CLR}\n{res_headers}\n\n{res_body}");
        },
        (_, _) => {
            println!(
                "{PURP}{request_line}{CLR}\n{req_headers}\n{req_body}\n\n\
                {res_color}{status_line}{CLR}\n{res_headers}\n\n{res_body}");
        },
    }
}

fn show_help() {
    let prog_name = env!("CARGO_BIN_NAME");

    eprintln!("\
{GRN}Usage:{CLR} {prog_name} <URI> [DATA]\n
{GRN}Arguments:{CLR}
    URI    An HTTP URI to a remote host (e.g. \"httpbin.org/json\").
    DATA   Data to be sent in the request body (optional).\n
{GRN}Options:{CLR}
    --help           Displays this help message.
    --method METHOD  Use METHOD as the request method (default: \"GET\").
    --path PATH      Use PATH as the URI path (default: \"/\").
    --tui            Starts the client TUI.\n
{GRN}Test Options:{CLR}
    --client-tests   Use output style expected by client tests.
    --server-tests   Use output style expected by server tests.");
}

use std::{env, io};

use rustnet::{Client, consts::DATE};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const PURP: &str = "\x1b[95m";
const CYAN: &str = "\x1b[96m";
const CLR: &str = "\x1b[0m";

#[rustfmt::skip]
fn main() -> io::Result<()> {
    let mut use_color_output = false;
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
                    eprintln!("{RED}Must provide a URL or IP address.{CLR}\n");
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
                "--colorize" => use_color_output = true,
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
                eprintln!("{RED}Must provide a URL or IP address.{CLR}\n");
                show_help();
                return Ok(());
            },
        }
    };

    // Process the remainder of the arguments or use default values.
    let addr = if is_testing { addr } else { format!("{addr}:80") };
    let uri = args.next().unwrap_or_else(|| String::from("/"));
    let body = args.next().unwrap_or_default();

    // Create an HTTP client and send a request.
    let mut client = Client::http()
        .addr(&addr)
        .uri(&uri)
        .body(body.as_bytes())
        .send()?;

    // Receive the response from the server.
    let mut res = client.recv()?;

    if is_testing {
        let _ = res.headers.remove(&DATE);
        println!("{client}{res}");
    } else if use_color_output {
        println!("{PURP}--[Request]-->\n{client}{CLR}\n");
        println!("{CYAN}<--[Response]--\n{res}{CLR}");
    } else {
        println!("{client}{res}");
    }

    Ok(())
}

fn show_help() {
    let help_msg = format!("\
        {GRN}USAGE{CLR}\n    \
            client <addr> [uri] [body]\n\n\
        {GRN}ARGUMENTS{CLR}\n    \
            addr    A remote host's URL or IP address (e.g. \"httpbin.org\").\n    \
            uri     Target URI on the remote host (default: \"/\").\n    \
            body    Data to be sent in the request body (optional).\n\n\
        {GRN}OPTIONS{CLR}\n    \
            --colorize    Prints colored output to the terminal.\n    \
            -h, --help    Displays this help message.\n    \
            --testing     The Date and Host headers are stripped from the response.\
    ");

    eprintln!("{help_msg}");
}

use std::env::Args;
use std::process;
use std::str::FromStr;

use crate::{
    Body, Client, Headers, Method, OutputStyle, Tui, WriteCliError,
};
use crate::colors::{CLR, GRN};
use crate::util;

/// Contains the parsed client command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientCli {
    pub debug: bool,
    pub do_send: bool,
    pub method: Method,
    pub path: String,
    pub addr: String,
    pub headers: Headers,
    pub body: Body,
    pub output: OutputStyle,
}

impl Default for ClientCli {
    fn default() -> Self {
        Self {
            debug: false,
            do_send: true,
            method: Method::Get,
            path: String::new(),
            addr: String::new(),
            headers: Headers::new(),
            body: Body::Empty,
            output: OutputStyle::default()
        }
    }
}

impl WriteCliError for ClientCli {}

impl ClientCli {
    /// Returns a default `ClientCli` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Prints the client help message and exits the program.
    pub fn print_help(&self) {
        eprintln!("\
{GRN}USAGE:{CLR}
    http_client [OPTIONS] [--] <URI>\n
{GRN}ARGUMENT:{CLR}
    URI   An HTTP URI (e.g. \"httpbin.org/json\")\n
{GRN}OPTIONS:{CLR}
    --debug             Print debug information.
    --header HEADER     Add a header with format NAME:VALUE to the request.
    --help              Display this help message.
    --method METHOD     Use METHOD as the request method (default: \"GET\").
    --minimal           Only output the status line.
    --no-dates          Remove Date headers from the output (useful during testing).
    --output FORMAT     Set the output to FORMAT (default: --output=\"shb\").
    --path PATH         Use PATH as the URI path (default: \"/\").
    --plain             Do not colorize output.
    --request           Output the full request without sending it.
    --shutdown          Sends a SHUTDOWN request.
    --text TEXT         Send TEXT in the request body.
    --tui               Start the client TUI.
    --verbose           Output the full request and response.\n
{GRN}FORMAT OPTIONS:{CLR}
    R = request line
    H = request headers
    B = request body
    s = status line
    h = response headers
    b = response body\n");

        process::exit(0);
    }

    /// Parses the command line options into a `ClientCli` object.
    pub fn parse_args(args: &mut Args) -> Self {
        let mut cli = Self::new();

        let _ = args.next();

        while let Some(opt) = args.next().as_deref() {
            if !opt.starts_with("--") {
                // First non-option argument is the URI argument.
                cli.handle_uri(opt);
                return cli;
            }

            match opt.len() {
                2 if opt == "--" => match args.next().as_deref() {
                    // URI following the end of options flag.
                    Some(uri) => cli.handle_uri(uri),
                    None => cli.missing_arg("URI"),
                },
                // Run the client TUI.
                5 if opt == "--tui" => Tui::run(),
                6 => match opt {
                    // Print the help message.
                    "--help" => cli.print_help(),
                    // Only print the response body.
                    "--body" => cli.output.format_str("b"),
                    // Set request body data.
                    "--text" => match args.next().as_deref() {
                        Some(text) => cli.handle_body(text),
                        None => cli.missing_arg(opt),
                    },
                    // Path component of the requested HTTP URI.
                    "--path" => match args.next().as_deref() {
                        Some(path) => cli.path = path.to_string(),
                        None => cli.missing_arg(opt),
                    },
                    _ => cli.unknown_opt(opt),
                },
                7 => match opt {
                    // Enable debugging.
                    "--debug" => cli.debug = true,
                    // Do not colorize output.
                    "--plain" => cli.output.make_plain(),
                    _ => cli.unknown_opt(opt),
                },
                8 => match opt {
                    // Set request method.
                    "--method" => match args.next().as_deref() {
                        Some(method) => cli.handle_method(method),
                        None => cli.missing_arg(opt),
                    },
                    // Add a request header.
                    "--header" => match args.next().as_deref() {
                        Some(header) => cli.handle_header(header),
                        None => cli.missing_arg(opt),
                    },
                    // Set the output style.
                    "--output" => match args.next().as_deref() {
                        Some(format) => cli.output.format_str(format),
                        None => cli.missing_arg(opt),
                    },
                    _ => cli.unknown_opt(opt),
                },
                9 => match opt {
                    // Only print the status line.
                    "--minimal" => cli.output.format_str("s"),
                    // Set verbose output style.
                    "--verbose" => cli.output.format_str("RHBshb"),
                    // Set the request output style and do not send.
                    "--request" => {
                        cli.do_send = false;
                        cli.output.format_str("RHB");
                    },
                    _ => cli.unknown_opt(opt),
                },
                10 => match opt {
                    // Remove Date headers before printing.
                    "--no-dates" => cli.output.no_dates = true,
                    // Set request output style and no send option.
                    "--shutdown" => cli.do_shutdown(args),
                    _ => cli.unknown_opt(opt),
                },
                // Handle an unknown option.
                _ => cli.unknown_opt(opt),
            }
        }

        if cli.addr.is_empty() {
            cli.missing_arg("URI");
        }

        cli
    }

    fn handle_method(&mut self, method: &str) {
        let method = method.to_ascii_uppercase();

        let Ok(method) = Method::from_str(method.as_str()) else {
            return self.invalid_arg("--method", &method);
        };

        self.method = method;
    }

    fn handle_header(&mut self, header: &str) {
        match header.split_once(':') {
            Some((name, value)) => self.headers.header(name, value),
            None => self.invalid_arg("--header", header),
        }
    }

    fn handle_body(&mut self, body: &str) {
        self.body = Body::Text(Vec::from(body));

        if let Some(con_type) = self.body.as_content_type() {
            self.headers.content_type(con_type);
        }

        self.headers.content_length(self.body.len());
    }

    fn handle_uri(&mut self, uri: &str) {
        match util::parse_uri(uri).ok() {
            Some((addr, path)) if self.path.is_empty() => {
                self.addr = addr;
                self.path = path;
            },
            // Do not clobber a previously set path.
            Some((addr, _path)) => self.addr = addr,
            None => self.invalid_arg("URI", uri),
        }
    }

    fn do_shutdown(&self, args: &mut Args) {
        let Some(addr) = args.by_ref().last() else {
            return self.missing_arg("URI");
        };

        if let Err(e) = Client::shutdown(&addr) {
            eprintln!("Could not send the shutdown request.\n{e}");
        }

        process::exit(0);
    }
}

use std::env::Args;
use std::process;

use crate::{Headers, Method};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const CLR: &str = "\x1b[0m";

/// Contains the parsed server command line arguments.
#[derive(Debug)]
pub struct ServerCli {
    pub log: bool,
    pub shutdown_route: bool,
    pub addr: String,
}

impl Default for ServerCli {
    fn default() -> Self {
        Self {
            log: false,
            shutdown_route: false,
            addr: "127.0.0.1:7878".to_string(),
        }
    }
}

impl ServerCli {
    /// Returns a default `ServerCli` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Prints the server binary's help message to the terminal.
    pub fn print_help(&self) {
        eprintln!(
            "\
{GRN}USAGE:{CLR}
    http_server [OPTIONS] <SERVER ADDRESS>\n
{GRN}SERVER ADDRESS:{CLR}
    IP:PORT    The server's IP address and port (default: 127.0.0.1:7878).\n
{GRN}OPTIONS:{CLR}
    --log             Enables logging of connections to the terminal.
    --shutdown-route  Adds a server shutdown route for testing."
        );
    }

    /// Parses command line arguments into a `ServerCli` object.
    #[must_use]
    pub fn parse(args: Args) -> Self {
        let mut cli = Self::new();
        let mut args = args.skip(1);

        while let Some(opt) = args.next().as_deref() {
            if opt.is_empty() {
                break;
            }

            match opt {
                "--log" => cli.log = true,
                "--shutdown-route" => cli.shutdown_route = true,
                "--help" => {
                    cli.print_help();
                    process::exit(0);
                },
                "--" => {
                    if let Some(addr) = args.next().as_deref() {
                        cli.addr = addr.to_string();
                        break;
                    }
                },
                unk if unk.starts_with("--") => {
                    eprintln!("Unknown option: \"{unk}\"\n");
                    process::exit(1);
                },
                addr => {
                    cli.addr = addr.to_string();
                    break;
                },
            }
        }

        cli
    }
}

/// Contains the parsed client command line arguments.
#[derive(Debug)]
pub struct ClientCli {
    pub method: Method,
    pub path: Option<&'static str>,
    pub uri: &'static str,
    pub headers: Headers,
    pub data: Option<&'static str>,
    pub tui: bool,
    pub plain: bool,
    pub no_dates: bool,
    pub no_send: bool,
    pub request_line: bool,
    pub req_headers: bool,
    pub req_body: bool,
    pub status_line: bool,
    pub res_headers: bool,
    pub res_body: bool,
}

impl Default for ClientCli {
    fn default() -> Self {
        Self {
            method: Method::Get,
            path: None,
            uri: "",
            headers: Headers::new(),
            data: None,
            tui: false,
            plain: false,
            no_dates: false,
            no_send: false,
            request_line: false,
            req_headers: false,
            req_body: false,
            status_line: true,
            res_headers: true,
            res_body: true
        }
    }
}

impl ClientCli {
    /// Returns a default `ClientCli` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Prints unknown option error message and exits the program.
    pub fn handle_unknown_opt(&self, opt: &str) {
        eprintln!("{RED}Unknown option: \"{opt}\".{CLR}");
        process::exit(1);
    }

    /// Prints invalid argument error message and exits the program.
    pub fn handle_invalid_arg(&self, opt: &str) {
        eprintln!("{RED}Invalid argument to \"{opt}\".{CLR}");
        process::exit(1);
    }

    /// Prints missing option argument error message and exits the program.
    pub fn handle_missing_arg(&self, opt: &str) {
        eprintln!("{RED}Missing required argument to \"{opt}\".{CLR}");
        process::exit(1);
    }

    /// Prints the help message and exits the program.
    pub fn print_help(&self) {
        eprintln!(
            "\
{GRN}USAGE:{CLR} http_client [OPTIONS] <URI>\n
{GRN}ARGUMENT:{CLR}
    URI    An HTTP URI (e.g. \"httpbin.org/json\")\n.
{GRN}OPTIONS:{CLR}
    --body              Output the response body (same as --output=\"b\").
    --data DATA         Send DATA in the request body.
    --header NAME:VALUE Add a header to the request.
    --help              Displays this help message.
    --method METHOD     Use METHOD as the request method (default: \"GET\").
    --minimal           Outputs the request line and the response's status
                        line (same as --output=\"Rs\").
    --no-dates          Remove Date headers from output (useful during testing).
    --output FORMAT     Set the output style (default: --output=\"shb\").
                        See the FORMAT options below.
    --path PATH         Use PATH as the URI path (default: \"/\").
    --plain             Do not colorize output.
    --request           Output the full request but do not send it.
    --tui               Starts the client TUI.
    --verbose           Outputs full requests and responses (same as 
                        --output=\"RHBshb\").\n
{GRN}FORMAT:{CLR}
    R = request line
    H = request headers
    B = request body
    s = response status line
    h = response headers
    b = response body
    c = client tests
    z = server tests"
        );

        process::exit(0);
    }

    pub fn handle_output_arg(&mut self, arg: &str) {
        arg.chars().for_each(|c| match c {
            'R' => self.request_line = true,
            'H' => self.req_headers = true,
            'B' => self.req_body = true,
            's' => self.status_line = true,
            'h' => self.res_headers = true,
            'b' => self.res_body = true,
            'c' => {
                self.request_line = true;
                self.req_headers = true;
                self.req_body = false;
                self.status_line = false;
                self.res_headers = false;
                self.res_body = false;
                self.plain = true;
                self.no_dates = true;
                return;
            },
            'z' => {
                self.request_line = false;
                self.req_headers = false;
                self.req_body = false;
                self.status_line = true;
                self.res_headers = true;
                self.res_body = true;
                self.plain = true;
                self.no_dates = true;
                return;
            },
            'r' => {
                self.request_line = true;
                self.req_headers = true;
                self.req_body = true;
                self.status_line = false;
                self.res_headers = false;
                self.res_body = false;
                self.no_send = true;
                return;
            },
            '*' => {
                self.request_line = true;
                self.req_headers = true;
                self.req_body = true;
                self.status_line = true;
                self.res_headers = true;
                self.res_body = true;
                return;
            },
            // Ignore quotation marks.
            '\'' | '"' => {},
            _ => self.handle_invalid_arg("--output"),
        });
    }

    /// Parses command line arguments into a `ClientCli` object.
    #[must_use]
    pub fn parse(args: Args) -> Self {
        let mut cli = Self::new();
        let mut args = args.skip(1);

        while let Some(opt) = args.next().as_deref() {
            match opt {
                // End of options flag.
                "--" => {
                    let optarg = args.next();
                    if let Some(uri) = optarg.as_deref() {
                        cli.uri = uri;
                    }

                    break;
                },
                // Print help message.
                "--help" => cli.print_help(),
                // Only print response bodies.
                "--body" => cli.handle_output_arg("b"),
                // Use the TUI.
                "--tui" => cli.tui = true,
                // Do not colorize.
                "--plain" => cli.plain = true,
                // Remove Date headers before printing.
                "--no-dates" => cli.no_dates = true,
                // Add a request header.
                "--header" => {
                    let optarg = args.next();
                    if let Some(header) = optarg.as_deref() {
                        if let Some((name, value)) = header.split_once(':') {
                            cli.headers.add_header(name, value);
                        } else {
                            cli.handle_invalid_arg("--header");
                        }
                    } else {
                        cli.handle_missing_arg("--header");
                    }
                },
                // Set request body data.
                "--data" => {
                    let optarg = args.next();
                    if let Some(data) = optarg.as_deref() {
                        cli.data = Some(data);
                    } else {
                        cli.handle_missing_arg("--data");
                    }
                },
                // Set request method.
                "--method" => {
                    let optarg = args.next();
                    if let Some(method) = optarg.as_ref() {
                        let method = method.to_ascii_uppercase();

                        if let Ok(custom_method) = method.parse::<Method>() {
                            cli.method = custom_method;
                        } else {
                            cli.handle_invalid_arg("--method");
                        }
                    } else {
                        cli.handle_missing_arg("--method");
                    }
                },
                // Path component of the requested HTTP URI.
                "--path" => {
                    let optarg = args.next();
                    if let Some(path) = optarg.as_deref() {
                        cli.path = Some(path);
                    } else {
                        cli.handle_missing_arg("--path");
                    }
                },
                // Set the output style.
                "--output" => {
                    let optarg = args.next();
                    if let Some(format_str) = optarg.as_deref() {
                        cli.handle_output_arg(format_str);
                    } else {
                        cli.handle_missing_arg("--output");
                    }
                },
                // Set request output style and no send option.
                "--request" => cli.handle_output_arg("RHB"),
                // Set verbose output style.
                "--verbose" => cli.handle_output_arg("*"),
                // Handle an unknown option.
                unk if unk.starts_with("--") => cli.handle_unknown_opt(unk),
                // First non-option argument should be the URI argument.
                uri => {
                    cli.uri = uri;
                    break;
                },
            }
        }

        if cli.uri.is_empty() {
            eprintln!("{RED}Missing required URI argument.{CLR}\n");
            process::exit(1);
        }

        cli
    }
}

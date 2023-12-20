use std::env::Args;
use std::process;

use crate::{Headers, Method, NetResult};
use crate::util;

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const CLR: &str = "\x1b[0m";

/// Contains the parsed server command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    pub fn parse_args(args: Args) -> Self {
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
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientCli {
    pub method: Method,
    pub path: String,
    pub addr: String,
    pub headers: Headers,
    pub data: Option<String>,
    pub do_send: bool,
    pub use_color: bool,
    pub no_dates: bool,
    pub tui: bool,
    pub out_req_line: bool,
    pub out_req_headers: bool,
    pub out_req_body: bool,
    pub out_status_line: bool,
    pub out_res_headers: bool,
    pub out_res_body: bool,
}

impl Default for ClientCli {
    fn default() -> Self {
        Self {
            method: Method::Get,
            path: String::new(),
            addr: String::new(),
            headers: Headers::new(),
            data: None,
            do_send: true,
            use_color: true,
            no_dates: false,
            tui: false,
            out_req_line: false,
            out_req_headers: false,
            out_req_body: false,
            out_status_line: true,
            out_res_headers: true,
            out_res_body: true
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
{GRN}USAGE:{CLR}
    http_client [OPTIONS] <URI>\n
{GRN}ARGUMENT:{CLR}
    URI   An HTTP URI (e.g. \"httpbin.org/json\")\n
{GRN}OPTIONS:{CLR}
    --body              Only output the response body.
    --data DATA         Add DATA to the request body.
    --header HEADER     Add a header with format NAME:VALUE to the request.
    --help              Display this help message.
    --method METHOD     Use METHOD as the request method (default: \"GET\").
    --minimal           Only output the request line and status line.
    --no-dates          Remove Date headers from the output (useful during testing).
    --output FORMAT     Set the output to FORMAT (default: --output=\"shb\").
    --path PATH         Use PATH as the URI path (default: \"/\").
    --plain             Do not colorize output.
    --request           Output the full request without sending it.
    --tui               Start the client TUI.
    --verbose           Output the full request and response.\n
{GRN}FORMAT OPTIONS:{CLR}
    R = request line
    H = request headers
    B = request body
    s = response status line
    h = response headers
    b = response body
    c = client tests
    z = server tests\n");

        process::exit(0);
    }

    pub fn handle_output_arg(&mut self, arg: &str) {
        // Disable default output style first.
        self.out_status_line = false;
        self.out_res_headers = false;
        self.out_res_body = false;

        arg.chars().for_each(|c| match c {
            'R' => self.out_req_line = true,
            'H' => self.out_req_headers = true,
            'B' => self.out_req_body = true,
            's' => self.out_status_line = true,
            'h' => self.out_res_headers = true,
            'b' => self.out_res_body = true,
            'c' => {
                self.out_req_line = true;
                self.out_req_headers = true;
                self.out_req_body = false;
                self.out_status_line = true;
                self.out_res_headers = true;
                self.out_res_body = false;
                self.use_color = false;
                self.no_dates = true;
            },
            'z' => {
                self.out_req_line = false;
                self.out_req_headers = false;
                self.out_req_body = false;
                self.out_status_line = true;
                self.out_res_headers = true;
                self.out_res_body = true;
                self.use_color = false;
                self.no_dates = true;
            },
            'r' => {
                self.out_req_line = true;
                self.out_req_headers = true;
                self.out_req_body = true;
                self.out_status_line = false;
                self.out_res_headers = false;
                self.out_res_body = false;
                self.do_send = false;
            },
            '*' => {
                self.out_req_line = true;
                self.out_req_headers = true;
                self.out_req_body = true;
                self.out_status_line = true;
                self.out_res_headers = true;
                self.out_res_body = true;
            },
            // Ignore quotation marks.
            '\'' | '"' => {},
            _ => self.handle_invalid_arg("--output"),
        });
    }

    /// Parses command line arguments into a `ClientCli` object.
    #[allow(clippy::missing_errors_doc)]
    pub fn parse_args(args: Args) -> NetResult<Self> {
        let mut cli = Self::new();
        let mut args = args.skip(1);

        while let Some(ref opt) = args.next() {
            match opt.as_str() {
                // End of options flag.
                "--" => {
                    if let Some(ref uri) = args.next() {
                        // Parse the URI argument.
                        let (addr, path) = util::parse_uri(uri)?;

                        cli.addr = addr;

                        if cli.path.is_empty() {
                            cli.path = path;
                        }
                    }
                    break;
                },
                // Print help message.
                "--help" => cli.print_help(),
                // Only print request lines and status lines.
                "--minimal" => cli.handle_output_arg("Rs"),
                // Only print response bodies.
                "--body" => cli.handle_output_arg("b"),
                // Use the TUI.
                "--tui" => {
                    cli.tui = true;
                    break;
                },
                // Do not colorize.
                "--plain" => cli.use_color = false,
                // Remove Date headers before printing.
                "--no-dates" => cli.no_dates = true,
                // Add a request header.
                "--header" => {
                    if let Some(ref header) = args.next() {
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
                    if let Some(ref data) = args.next() {
                        cli.data = Some(data.to_string());
                    } else {
                        cli.handle_missing_arg("--data");
                    }
                },
                // Set request method.
                "--method" => {
                    if let Some(ref method) = args.next() {
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
                    if let Some(ref path) = args.next() {
                        cli.path = path.to_string();
                    } else {
                        cli.handle_missing_arg("--path");
                    }
                },
                // Set the output style.
                "--output" => {
                    if let Some(ref format_str) = args.next() {
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
                    let (addr, path) = util::parse_uri(uri)?;
                    if cli.path.is_empty() {
                        cli.path = path;
                    }

                    cli.addr = addr;
                    break;
                },
            }
        }

        if cli.addr.is_empty() && !cli.tui {
            eprintln!("{RED}Missing required URI argument.{CLR}\n");
            process::exit(1);
        }

        Ok(cli)
    }
}

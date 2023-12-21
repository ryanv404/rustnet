use std::env::Args;
use std::path::PathBuf;
use std::process;

use crate::{
    Headers, Method, NetError, NetParseError, NetResult, Route, Router,
    Target,
};
use crate::util;

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const CLR: &str = "\x1b[0m";

/// Contains the parsed server command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerCli {
    pub is_test: bool,
    pub do_logging: bool,
    pub addr: String,
    pub router: Router,
}

impl Default for ServerCli {
    fn default() -> Self {
        Self {
            is_test: false,
            do_logging: false,
            addr: String::new(),
            router: Router::new()
        }
    }
}

impl ServerCli {
    /// Returns a default `ServerCli` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Prints unknown option error message and exits the program.
    pub fn handle_unknown_opt(&self, opt_name: &str) {
        eprintln!("{RED}Unknown option: \"{opt_name}\".{CLR}");
        process::exit(1);
    }

    /// Prints missing option argument error message and exits the program.
    pub fn handle_missing_optarg(&self, opt_name: &str) {
        eprintln!("{RED}Missing required argument to \"{opt_name}\".{CLR}");
        process::exit(1);
    }

    pub fn handle_invalid_optarg(&self, opt_name: &str) {
        eprintln!("{RED}Invalid argument to \"{opt_name}\".{CLR}");
        process::exit(1);
    }

    /// Prints missing CLI argument error message and exits the program.
    pub fn handle_missing_arg(&self, arg_name: &str) {
        eprintln!("{RED}Missing required {arg_name} argument.{CLR}");
        process::exit(1);
    }

    /// Prints invalid CLI argument error message and exits the program.
    pub fn handle_invalid_arg(&self, arg_name: &str, bad_arg: &str) {
        eprintln!("{RED}Invalid {arg_name} argument: \"{bad_arg}\"{CLR}");
        process::exit(1);
    }

    /// Parses and inserts a route into the server's router.
    ///
    /// # Errors
    /// 
    /// Returns an error if parsing of the input into a valid route fails.
    pub fn handle_route(&mut self, route: &str, kind: &str) -> NetResult<()> {
        let mut tokens = route.splitn(2, ':');

        match (kind, tokens.next()) {
            (_, None) => self.handle_invalid_optarg("--route"),
            ("favicon", Some(filepath)) => {
                let route = Route::Get("/favicon.ico".into());
                let target = Target::Favicon(PathBuf::from(filepath));
                self.router.mount(route, target);
            },
            ("404", Some(filepath)) => {
                let route = Route::NotFound;
                let target = Target::File(PathBuf::from(filepath));
                self.router.mount(route, target);
            },
            ("file" | "text", Some(method)) => {
                let method = method.parse::<Method>()?;

                let path = tokens
                    .next()
                    .ok_or(NetError::Parse(NetParseError::UriPath))
                    .map(ToString::to_string)?;

                let route = Route::new(method, &path);

                let target = if kind == "text" {
                    tokens
                        .next()
                        .ok_or(NetError::Other("route text parsing"))
                        .map(|text| Target::Text(text.into()))?
                } else {
                    tokens
                        .next()
                        .ok_or(NetError::Other("route file path parsing"))
                        .map(|fpath| Target::File(PathBuf::from(fpath)))?
                };

                self.router.mount(route, target);
            },
            (_, _) => unreachable!(),
        }

        Ok(())
    }

    /// Prints the server help message and exists the program.
    pub fn print_help(&self) {
        eprintln!(
            "\
{GRN}USAGE:{CLR}
    http_server [OPTIONS] [--] <SERVER ADDRESS>\n
{GRN}SERVER ADDRESS:{CLR}
    IP:PORT    The server's IP address and port.\n
{GRN}OPTIONS:{CLR}
    --log      Enables logging of connections to the terminal.
    --test     Creates a test server with a shutdown route.\n
{GRN}ROUTES:{CLR}
    --file-route METHOD:URI_PATH:FILE_PATH
            Add a route with the given HTTP method that serves a file.
    --text-route METHOD:URI_PATH:TEXT
            Add a route with the given HTTP method that serves text.
    --favicon-route FILE_PATH
            Add a favicon.
    --404-route FILE_PATH
            Add a static file that is returned with 404 responses.\n"
        );

        process::exit(0);
    }

    /// Parses command line arguments into a `ServerCli` object.
    #[must_use]
    pub fn parse_args(args: Args) -> Self {
        let mut cli = Self::new();

        let mut args = args.skip(1);

        while let Some(arg) = args.next() {
            match arg.len() {
                // Enable logging of new connections.
                5 if arg == "--log" => cli.do_logging = true,
                // Make the server a test server.
                6 if arg == "--test" => cli.is_test = true,
                // Print help message.
                6 if arg == "--help" => cli.print_help(),
                // Add a route that serves a file.
                12 if arg == "--file-route" => match args.next().as_deref() {
                    Some(route) => {
                        if cli.handle_route(route, "file").is_err() {
                            cli.handle_invalid_optarg("--route");
                        }
                    },
                    None => cli.handle_missing_optarg("--route"),
                },
                // Add a route that serves text.
                12 if arg == "--text-route" => match args.next().as_deref() {
                    Some(route) => {
                        if cli.handle_route(route, "text").is_err() {
                            cli.handle_invalid_optarg("--route");
                        }
                    },
                    None => cli.handle_missing_optarg("--route"),
                },
                // Add a favicon route.
                15 if arg == "--favicon-route" => match args.next().as_deref() {
                    Some(route) => {
                        if cli.handle_route(route, "favicon").is_err() {
                            cli.handle_invalid_optarg("--route");
                        }
                    },
                    None => cli.handle_missing_optarg("--route"),
                },
                // Set a file to serve for routes that are not found.
                11 if arg == "--404-route" => match args.next().as_deref() {
                    Some(route) => {
                        if cli.handle_route(route, "404").is_err() {
                            cli.handle_invalid_optarg("--route");
                        }
                    },
                    None => cli.handle_missing_optarg("--route"),
                },
                // End of options flag.
                2 if arg == "--" => match args.next() {
                    // First non-option argument is the server address.
                    Some(addr) => {
                        cli.addr = addr;
                        break;
                    },
                    None => cli.handle_missing_arg("server address"),
                },
                // Unknown option.
                _ if arg.starts_with("--") => cli.handle_unknown_opt(&arg),
                // First non-option argument is the server address.
                _ => {
                    cli.addr = arg;
                    break;
                },
            }
        }

        if cli.addr.is_empty() {
            cli.handle_missing_arg("server address");
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
    pub fn handle_unknown_opt(&self, opt_name: &str) {
        eprintln!("{RED}Unknown option: \"{opt_name}\".{CLR}");
        process::exit(1);
    }

    /// Prints missing option argument error message and exits the program.
    pub fn handle_missing_optarg(&self, opt_name: &str) {
        eprintln!("{RED}Missing required argument to \"{opt_name}\".{CLR}");
        process::exit(1);
    }

    /// Prints invalid argument error message and exits the program.
    pub fn handle_invalid_optarg(&self, opt_name: &str) {
        eprintln!("{RED}Invalid argument to \"{opt_name}\".{CLR}");
        process::exit(1);
    }

    /// Prints missing CLI argument error message and exits the program.
    pub fn handle_missing_arg(&self, arg_name: &str) {
        eprintln!("{RED}Missing required {arg_name} argument.{CLR}");
        process::exit(1);
    }

    /// Prints invalid CLI argument error message and exits the program.
    pub fn handle_invalid_arg(&self, arg_name: &str, bad_arg: &str) {
        eprintln!("{RED}Invalid {arg_name} argument: \"{bad_arg}\"{CLR}");
        process::exit(1);
    }

    /// Prints the client help message and exits the program.
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

    pub fn handle_style(&mut self, arg: &str) {
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
            _ => self.handle_invalid_optarg("--output"),
        });
    }

    /// Parses command line arguments into a `ClientCli` object.
    #[allow(clippy::missing_errors_doc)]
    pub fn parse_args(args: Args) -> NetResult<Self> {
        let mut cli = Self::new();

        let mut args = args.skip(1);

        while let Some(opt) = args.next().as_deref() {
            match opt {
                // End of options flag.
                "--" => match args.next().as_deref() {
                    Some(uri) => {
                        // First non-option argument should be the URI argument.
                        cli.parse_uri(uri);
                        break;
                    },
                    None => cli.handle_missing_arg("URI"),
                },
                // Print help message.
                "--help" => cli.print_help(),
                // Only print request lines and status lines.
                "--minimal" => cli.handle_style("Rs"),
                // Only print response bodies.
                "--body" => cli.handle_style("b"),
                // Run the client TUI.
                "--tui" => {
                    cli.tui = true;
                    return Ok(cli);
                },
                // Do not colorize.
                "--plain" => cli.use_color = false,
                // Remove Date headers before printing.
                "--no-dates" => cli.no_dates = true,
                // Add a request header.
                "--header" => {
                    if let Some(ref header) = args.next() {
                        if let Some((name, value)) = header.split_once(':') {
                            cli.headers.header(name, value);
                        } else {
                            cli.handle_invalid_optarg("--header");
                        }
                    } else {
                        cli.handle_missing_optarg("--header");
                    }
                },
                // Set request body data.
                "--data" => {
                    if let Some(data) = args.next().as_deref() {
                        cli.data = Some(data.to_string());
                    } else {
                        cli.handle_missing_optarg("--data");
                    }
                },
                // Set request method.
                "--method" => {
                    if let Some(method) = args.next().as_deref() {
                        let method = method.to_ascii_uppercase();

                        if let Ok(custom_method) = method.parse::<Method>() {
                            cli.method = custom_method;
                        } else {
                            cli.handle_invalid_optarg("--method");
                        }
                    } else {
                        cli.handle_missing_optarg("--method");
                    }
                },
                // Path component of the requested HTTP URI.
                "--path" => {
                    if let Some(path) = args.next().as_deref() {
                        cli.path = path.to_string();
                    } else {
                        cli.handle_missing_optarg("--path");
                    }
                },
                // Set the output style.
                "--output" => {
                    if let Some(style) = args.next().as_deref() {
                        cli.handle_style(style);
                    } else {
                        cli.handle_missing_optarg("--output");
                    }
                },
                // Set request output style and no send option.
                "--request" => cli.handle_style("RHB"),
                // Set verbose output style.
                "--verbose" => cli.handle_style("*"),
                // Handle an unknown option.
                unk if unk.starts_with("--") => cli.handle_unknown_opt(unk),
                // First non-option argument should be the URI argument.
                uri => {
                    cli.parse_uri(uri);
                    break;
                },
            }
        }

        if cli.addr.is_empty() {
            cli.handle_missing_arg("URI");
        }

        Ok(cli)
    }

    pub fn parse_uri(&mut self, uri: &str) {
        match util::parse_uri(uri).ok() {
            Some((addr, path)) => {
                self.addr = addr;

                // Do not clobber a previously set path.
                if self.path.is_empty() {
                    self.path = path;
                }
            },
            None => self.handle_invalid_arg("URI", uri),
        }
    }
}

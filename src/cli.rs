use std::env::Args;
use std::path::PathBuf;
use std::process;

use crate::{
    Body, Client, Headers, Method, NetResult, OutputStyle, Route, Router,
    Target, Tui, WriteCliError,
};
use crate::colors::{CLR, GRN};
use crate::util;

/// Contains the parsed server command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerCli {
    pub debug: bool,
    pub do_log: bool,
    pub is_test_server: bool,
    pub addr: String,
    pub router: Router,
}

impl Default for ServerCli {
    fn default() -> Self {
        Self {
            debug: false,
            do_log: false,
            is_test_server: false,
            addr: String::new(),
            router: Router::new()
        }
    }
}

impl WriteCliError for ServerCli {}

impl ServerCli {
    /// Returns a default `ServerCli` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses and inserts a route into the server's router.
    fn parse_route(&mut self, opt: &str, arg: &str) {
        if arg.is_empty() {
            self.missing_arg(opt);
        }

        let mut tokens = arg.splitn(3, ':');

        let tokens = (
            tokens.next(), tokens.next(), tokens.next()
        );

        match opt {
            "--favicon" => match tokens.0 {
                Some(path) => self.router.insert_favicon(path),
                None => self.invalid_arg(opt, arg),
            },
            "--not-found" => match tokens.0 {
                Some(path) => self.router.insert_not_found(path),
                None => self.invalid_arg(opt, arg),
            },
            "--text" | "--file" => match tokens {
                (Some(method), Some(path), Some(target)) => {
                    let method = method.to_ascii_uppercase();
                    let method = Method::from(method.as_str());

                    let route = Route::new(&method, path);

                    let target = match opt {
                        "--text" => Target::Text(target.to_string()),
                        "--file" => Target::File(PathBuf::from(target)),
                        _ => unreachable!(),
                    };

                    self.router.mount(route, target);
                },
                (_, _, _) => self.invalid_arg(opt, &arg),
            },
            _ => self.unknown_opt(opt),
        }
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
    --debug      Prints debug information.
    --help       Prints this help message.
    --log        Enables logging of connections to stdout.
    --test       Creates a test server with a shutdown route.\n
{GRN}ROUTES:{CLR}
    --text METHOD:URI_PATH:TEXT
            Adds a route that serves text.
    --file METHOD:URI_PATH:FILE_PATH
            Adds a route that serves a file.
    --favicon FILE_PATH
            Adds a route that serves a favicon icon.
    --not-found FILE_PATH
            Adds a route that handles 404 Not Found responses.\n"
        );

        process::exit(0);
    }

    /// Parses command line arguments into a `ServerCli` object.
    #[must_use]
    pub fn parse_args(args: &mut Args) -> Self {
        let mut cli = Self::new();

        let _ = args.next();

        while let Some(opt) = args.next().as_deref() {
            if !opt.starts_with("--") {
                // First non-option argument is the server address.
                cli.addr = opt.to_string();
                return cli;
            }

            match opt.len() {
                // End of options flag.
                2 if opt == "--" => match args.next() {
                    // First non-option argument is the server address.
                    None => cli.missing_arg("SERVER ADDRESS"),
                    Some(addr) => {
                        cli.addr = addr;
                        break;
                    },
                },
                // Enable logging of new connections.
                5 if opt == "--log" => cli.do_log = true,
                6 => match opt {
                    // Print help message.
                    "--help" => cli.print_help(),
                    // Make the server a test server.
                    "--test" => {
                        cli.is_test_server = true;
                        cli.router.mount_shutdown_route();
                    },
                    // Add a route.
                    "--file" | "--text" => match args.next().as_deref() {
                        Some(arg) => cli.parse_route(opt, arg),
                        None => cli.missing_arg(opt),
                    },
                    _ => cli.unknown_opt(opt),
                },
                // Enable debugging.
                7 if opt == "--debug" => cli.debug = true,
                // Add a favicon route.
                9 | 11 => match (opt, args.next().as_deref()) {
                    ("--favicon" | "--not-found", Some(arg)) => {
                        cli.parse_route(opt, arg)
                    },
                    ("--favicon" | "--not-found", None) => {
                        cli.missing_arg(opt)
                    },
                    (_, _) => cli.unknown_opt(opt),
                },
                // Unknown option.
                _ => cli.unknown_opt(opt),
            }
        }

        if cli.addr.is_empty() {
            cli.missing_arg("SERVER ADDRESS");
        }

        cli
    }
}

/// Contains the parsed client command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientCli {
    pub debug: bool,
    pub do_not_send: bool,
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
            do_not_send: false,
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
    #[must_use]
    pub fn parse_args(args: &mut Args) -> NetResult<Self> {
        let mut cli = Self::new();

        let _ = args.next();

        while let Some(opt) = args.next().as_deref() {
            if !opt.starts_with("--") {
                // First non-option argument is the URI argument.
                cli.handle_uri(opt);
                return Ok(cli)
            }

            match opt.len() {
                2 if opt == "--" => match args.next().as_deref() {
                    // URI following the end of options flag.
                    Some(uri) => cli.handle_uri(uri),
                    None => cli.missing_arg("URI"),
                },
                // Run the client TUI.
                5 if opt == "--tui" => {
                    Tui::run();
                    process::exit(0);
                },
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
                        cli.do_not_send = true;
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

        Ok(cli)
    }

    fn handle_method(&mut self, method: &str) {
        let method = method.to_ascii_uppercase();
        self.method = Method::from(method.as_str());
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
        match args.by_ref().last() {
            None => self.missing_arg("URI"),
            Some(addr) => {
                if let Err(e) = Client::shutdown(&addr) {
                    eprintln!("Could not send the shutdown request.\n{e}");
                }

                process::exit(0);
            },
        }
    }
}

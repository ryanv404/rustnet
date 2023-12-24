use std::env::Args;
use std::iter::Skip;
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
    pub debug_cli: bool,
    pub do_logging: bool,
    pub is_test_server: bool,
    pub addr: String,
    pub router: Router,
}

impl Default for ServerCli {
    fn default() -> Self {
        Self {
            debug_cli: false,
            do_logging: false,
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
    pub fn parse_route(&mut self, opt: &str, arg: &str) {
        if arg.is_empty() {
            self.missing_arg(opt);
        }

        let mut tokens = arg.splitn(3, ':');
        let token1 = tokens.next();
        let token2 = tokens.next();
        let token3 = tokens.next();

        match opt {
            "--favicon-route" => match token1 {
                Some(path) => self.router.insert_favicon(path),
                None => self.invalid_arg(opt, arg),
            },
            "--not-found-route" => match token1 {
                Some(path) => self.router.insert_not_found(path),
                None => self.invalid_arg(opt, arg),
            },
            "--file-route" | "--text-route" => match (token1, token2, token3) {
                (Some(method), Some(path), Some(target)) => {
                    let uppercase = method.to_ascii_uppercase();
                    let method = Method::from(uppercase.as_str());

                    let route = Route::new(&method, path);

                    let target = if opt == "--text-route" {
                        Target::Text(target.to_string())
                    } else {
                        Target::File(PathBuf::from(target))
                    };

                    self.router.mount(route, target);
                },
                (_, _, _) => self.invalid_arg(opt, arg),
            },
            _ => unreachable!(),
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
    --debug-cli    Debug CLI parsing.
    --log          Enables logging of connections to stdout.
    --test-server  Creates a test server with a shutdown route.\n
{GRN}ROUTES:{CLR}
    --favicon-route FILE_PATH
            Adds a route that serves a favicon icon.
    --file-route METHOD:URI_PATH:FILE_PATH
            Adds a route that serves a file.
    --not-found-route FILE_PATH
            Adds a route that handles 404 Not Found responses.
    --text-route METHOD:URI_PATH:TEXT
            Adds a route that serves text.\n"
        );

        process::exit(0);
    }

    /// Parses command line arguments into a `ServerCli` object.
    #[must_use]
    pub fn parse_args(args: Args) -> Self {
        let mut cli = Self::new();
        let mut args = args.skip(1);

        while let Some(ref opt) = args.next() {
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
                5 if opt == "--log" => cli.do_logging = true,
                // Print help message.
                6 if opt == "--help" => cli.print_help(),
                // Enable debugging.
                11 if opt == "--debug-cli" => cli.debug_cli = true,
                // Add a route that serves a file.
                12 if opt == "--file-route" => match args.next() {
                    None => cli.missing_arg(opt),
                    Some(ref arg) => cli.parse_route(opt, arg),
                },
                // Add a route that serves text.
                12 if opt == "--text-route" => match args.next() {
                    None => cli.missing_arg(opt),
                    Some(ref arg) => cli.parse_route(opt, arg),
                },
                // Make the server a test server.
                13 if opt == "--test-server" => {
                    cli.is_test_server = true;
                    cli.router.mount_shutdown_route();
                },
                // Add a favicon route.
                15 if opt == "--favicon-route" => match args.next() {
                    None => cli.missing_arg(opt),
                    Some(ref arg) => cli.parse_route(opt, arg),
                },
                // Set a file to serve for routes that are not found.
                17 if opt == "--not-found-route" => match args.next() {
                    None => cli.missing_arg(opt),
                    Some(ref arg) => cli.parse_route(opt, arg),
                },
                // Unknown option.
                _ if opt.starts_with("--") => cli.unknown_arg(opt),
                // First non-option argument is the server address.
                _ => {
                    cli.addr = opt.to_string();
                    break;
                },
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

    pub fn do_shutdown(&self, args: Skip<Args>) {
        match args.last().as_deref() {
            Some(addr) => {
                if let Err(e) = Client::shutdown(addr) {
                    eprintln!("Could not send shutdown request.\n{e}");
                }

                process::exit(0);
            },
            None => self.missing_arg("URI"),
        }
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

    /// Parses command line arguments into a `ClientCli` object.
    #[must_use]
    pub fn parse_args(args: Args) -> NetResult<Self> {
        let mut cli = Self::new();

        let mut opts = args.skip(1);

        while let Some(ref opt) = opts.next() {
            match opt.len() {
                // End of options flag.
                2 if opt == "--" => {
                    cli.handle_uri(opts.next().as_deref());
                    break;
                },
                // Run the client TUI.
                5 if opt == "--tui" => {
                    Tui::run();
                    process::exit(0);
                },
                // Set request body data.
                6 if opt == "--text" => {
                    cli.handle_body(opts.next().as_deref());
                },
                // Path component of the requested HTTP URI.
                6 if opt == "--path" => match opts.next() {
                    None => cli.missing_arg("--path"),
                    Some(path) => cli.path = path,
                },
                // Print the help message.
                6 if opt == "--help" => cli.print_help(),
                // Only print the response body.
                6 if opt == "--body" => cli.output.format_str("b"),
                // Do not colorize output.
                7 if opt == "--plain" => cli.output.make_plain(),
                // Enable debugging.
                7 if opt == "--debug" => cli.debug = true,
                // Set request method.
                8 if opt == "--method" => {
                    cli.handle_method(opts.next().as_deref());
                },
                // Set the output style.
                8 if opt == "--output" => match opts.next() {
                    None => cli.missing_arg("--output"),
                    Some(ref format) => cli.output.format_str(format),
                },
                // Add a request header.
                8 if opt == "--header" => {
                    cli.handle_header(opts.next().as_deref());
                },
                // Only print the request line and status line.
                9 if opt == "--minimal" => cli.output.format_str("Rs"),
                // Set the request output style and the do_not_send option.
                9 if opt == "--request" => {
                    cli.do_not_send = true;
                    cli.output.format_str("RHB");
                },
                // Set verbose output style.
                9 if opt == "--verbose" => cli.output.format_str("RHBshb"),
                // Set request output style and no send option.
                10 if opt == "--shutdown" => {
                    cli.do_shutdown(opts);
                    break;
                },
                // Remove Date headers before printing.
                10 if opt == "--no-dates" => cli.output.no_dates = true,
                // Handle an unknown option.
                _ if opt.starts_with("--") => cli.unknown_arg(opt),
                // First non-option argument should be the URI argument.
                _ => {
                    cli.handle_uri(Some(opt));
                    break;
                },
            }
        }

        if cli.addr.is_empty() {
            cli.missing_arg("URI");
        }

        Ok(cli)
    }

    pub fn handle_method(&mut self, method: Option<&str>) {
        match method {
            None => self.missing_arg("--method"),
            Some(m_str) => {
                let m_str = m_str.to_ascii_uppercase();
                self.method = Method::from(m_str.as_str());
            },
        }
    }

    pub fn handle_header(&mut self, header: Option<&str>) {
        match header {
            None => self.missing_arg("--header"),
            Some(header) => match header.split_once(':') {
                None => self.invalid_arg("--header", header),
                Some((name, value)) => self.headers.header(name, value),
            },
        }
    }

    pub fn handle_body(&mut self, body: Option<&str>) {
        match body {
            None => self.missing_arg("--text"),
            Some(text) => {
                self.body = Body::Text(Vec::from(text));
                self.headers.content_length(self.body.len());

                if let Some(con_type) = self.body.as_content_type() {
                    self.headers.content_type(con_type);
                }
            },
        }
    }

    pub fn handle_uri(&mut self, uri: Option<&str>) {
        match uri {
            None => self.missing_arg("URI"),
            Some(uri) => match util::parse_uri(uri).ok() {
                None => self.invalid_arg("URI", uri),
                Some((addr, path)) => {
                    self.addr = addr;

                    // Do not clobber a previously set path.
                    if self.path.is_empty() {
                        self.path = path;
                    }
                },
            },
        }
    }
}

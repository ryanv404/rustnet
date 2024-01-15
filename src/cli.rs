use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::str::FromStr;

use crate::{
    Body, Client, Headers, Method, NetError, NetResult, Request, Route,
    Router, Server, Style, Target, Tui, UriPath, Version, WriteCliError,
    CLIENT_NAME, SERVER_NAME, TEST_SERVER_ADDR, utils,
};
use crate::style::colors::{GREEN, RED, RESET};

/// Contains the parsed client command line arguments.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientCli {
    pub do_send: bool,
    pub do_debug: bool,
    pub do_plain: bool,
    pub no_dates: bool,
    pub addr: Option<String>,
    pub style: Style,
    pub method: Method,
    pub path: UriPath,
    pub version: Version,
    pub headers: Headers,
    pub body: Body,
}

impl Default for ClientCli {
    fn default() -> Self {
        Self {
            do_send: true,
            do_debug: false,
            do_plain: false,
            no_dates: false,
            addr: None,
            style: Style::default(),
            method: Method::default(),
            path: UriPath::default(),
            version: Version::default(),
            headers: Headers::default(),
            body: Body::default(),
        }
    }
}

impl Display for ClientCli {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:?}")
    }
}

impl Debug for ClientCli {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(
            f,
            "ClientCli {{\n    \
            do_send: {:?},\n    \
            do_debug: {:?},\n    \
            do_plain: {:?},\n    \
            no_dates: {:?},",
            self.do_send,
            self.do_debug,
            self.do_plain,
            self.no_dates
        )?;

        if let Some(addr) = self.addr.as_ref() {
            writeln!(f, "    addr: Some({addr:?}),")?;
        } else {
            writeln!(f, "    addr: None,")?;
        }

        writeln!(
            f,
            "    style: {:?},\n    \
            method: {:?},\n    \
            path: {:?},\n    \
            version: {:?},",
            self.style,
            self.method,
            self.path,
            self.version
        )?;

        if self.headers.is_empty() {
            writeln!(f, "    headers: Headers(),")?;
        } else {
            writeln!(f, "    headers: Headers(")?;

            for (name, value) in &self.headers.0 {
                writeln!(f, "        {name:?}: {value:?},")?;
            }

            writeln!(f, "    ),")?;
        }

        writeln!(f, "    body: {:?}", self.body)?;
        write!(f, "}}")
    }
}

impl TryFrom<ClientCli> for Client {
    type Error = NetError;

    fn try_from(cli: ClientCli) -> NetResult<Self> {
        // Establish a connection.
        let Some(addr) = cli.addr.as_ref() else {
            return Err(NetError::NotConnected);
        };

        let req = Request {
            method: cli.method,
            path: cli.path.clone(),
            version: cli.version,
            headers: cli.headers.clone(),
            body: cli.body.clone()
        };

        let mut client = Self::builder()
            .do_send(cli.do_send)
            .do_debug(cli.do_debug)
            .no_dates(cli.no_dates)
            .style(cli.style)
            .req(req)
            .addr(addr)
            .build()?;

        if cli.do_plain {
            client.style.to_plain();
        }

        Ok(client)
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
{GREEN}USAGE:{RESET}
    {CLIENT_NAME} [OPTIONS] [--] <URI>\n
{GREEN}ARGUMENT:{RESET}
    URI     An HTTP URI (e.g. \"httpbin.org/json\").\n
{GREEN}OPTIONS:{RESET}
    -B, --body TEXT         Add TEXT to the request body.
    -b, --builder           Build a request and send it.
    -d, --debug             Print client debug information.
    -H, --header HEADER     Add a header with the format NAME:VALUE to the request.
    -h, --help              Display this help message.
    -M, --method METHOD     Use METHOD as the request method (default: \"GET\").
    -m, --minimal           Only print the request line and status line.
    -n, --no-dates          Remove Date headers from the output (used during testing).
    -O, --output FORMAT     Set the output style to FORMAT, see below
                            (default: --output \"shb\").
    -P, --path PATH         Use PATH as the URI path (default: \"/\").
    -p, --plain             Do not colorize the output.
    -r, --request           Print the request without sending it.
    -s, --server            Start a server listening on {TEST_SERVER_ADDR}.
    -S, --shutdown          Shut down the server running on {TEST_SERVER_ADDR}.
    -T, --tui               Run the client TUI.
    -v, --verbose           Print both the request and the response.
    -V, --version           Set the protocol version (default: \"HTTP/1.1\").\n
{GREEN}FORMAT OPTIONS:{RESET}
    R = request line        s = status line
    H = request headers     h = response headers
    B = request body        b = response body\n");

        process::exit(0);
    }

    /// Parses the command line options into a `Client`.
    ///
    /// # Errors
    ///
    /// Returns an error if a `Client` cannot be built.
    pub fn parse_args(args: &mut VecDeque<&str>) -> NetResult<Client> {
        let mut cli = Self::new();

        let _ = args.pop_front();

        if args.is_empty() {
            cli.print_help();
            process::exit(0);
        }

        while let Some(opt) = args.pop_front() {
            match opt {
                // URI is the first argument after "--".
                "--" => match args.pop_front() {
                    Some(arg) => {
                        cli.handle_uri(arg);
                        break;
                    },
                    None => cli.missing_arg("URI"),
                },
                // Run request builder.
                "-b" | "--builder" => {
                    let mut client = Client::default();
                    client.get_request_from_user()?;

                    // Set output style to "verbose".
                    client.style.from_format_str("*");

                    println!();
                    return Ok(client);
                },
                // Handle options.
                _ if opt.starts_with('-') => cli.handle_opt(opt, args),
                // URI is the first non-option argument.
                _ => cli.handle_uri(opt),
            }
        }

        if cli.do_plain {
            cli.style.to_plain();
        }

        Client::try_from(cli)
    }

    pub fn handle_opt(&mut self, opt: &str, args: &mut VecDeque<&str>) {
        match opt {
            // Run the client TUI.
            "-T" | "--tui" => Tui::run(),
            // Start a test server at localhost:7878.
            "-s" | "--server" => Self::start_server(),
            // Send a shutdown request to localhost:7878.
            "-S" | "--shutdown" => Self::do_shutdown(),
            // Print the help message.
            "-h" | "--help" => self.print_help(),
            // Do not colorize output.
            "-p" | "--plain" => self.do_plain = true,
            // Enable debug printing.
            "-d" | "--debug" => self.do_debug = true,
            // Remove Date headers before printing.
            "-n" | "--no-dates" => self.no_dates = true,
            // Only print the request line and status line.
            "-m" | "--minimal" => self.style.from_format_str("Rs"),
            // Set verbose output style.
            "-v" | "--verbose" => self.style.from_format_str("*"),
            // Set request output style and do not send.
            "-r" | "--request" => {
                self.do_send = false;
                self.style.from_format_str("RHB");
            },
            // Set the HTTP method.
            "-M" | "--method" => match args.pop_front() {
                Some(method) => self.handle_method(method),
                None => self.missing_arg(opt),
            },
            // Set the URI path.
            "-P" | "--path" => match args.pop_front() {
                Some(path) => {
                    self.path = path.to_string().into();
                },
                None => self.missing_arg(opt),
            },
            // Set the protocol version.
            "-V" | "--version" => match args.pop_front() {
                Some(version_str) => {
                    let version_str = version_str.to_ascii_uppercase();

                    match Version::from_str(version_str.trim()).ok() {
                        Some(version) => self.version = version,
                        None => self.invalid_arg(opt, &version_str),
                    }
                },
                None => self.missing_arg(opt),
            },
            // Add a request header.
            "-H" | "--header" => match args.pop_front() {
                Some(header) => self.handle_header(header),
                None => self.missing_arg(opt),
            },
            // Set the request body.
            "-B" | "--body" => match args.pop_front() {
                Some(body) => self.body = String::from(body.trim()).into(),
                None => self.missing_arg(opt),
            },
            // Set the output style based on a format string.
            "-O" | "--output" => match args.pop_front() {
                Some(format) => self.style.from_format_str(format.trim()),
                None => self.missing_arg(opt),
            },
            // Handle an unknown option.
            _ => self.unknown_opt(opt),
        }
    }

    pub fn handle_uri(&mut self, arg: &str) {
        match utils::parse_uri(arg).ok() {
            Some((addr, path)) if self.path.is_default() => {
                self.path = path.into();
                self.addr = Some(addr.trim().to_ascii_lowercase());
            },
            // Do not clobber a previously set path.
            Some((addr, _)) => {
                self.addr = Some(addr.trim().to_ascii_lowercase());
            },
            None => self.invalid_arg("URI", arg),
        }
    }

    pub fn handle_method(&mut self, method: &str) {
        let uppercase = method.trim().to_ascii_uppercase();

        match Method::from_str(uppercase.as_str()).ok() {
            Some(method) => self.method = method,
            None => self.invalid_arg("--method", method),
        }
    }

    pub fn handle_header(&mut self, header: &str) {
        let Some((name, value)) = header.split_once(':') else {
            return self.invalid_arg("--header", header);
        };

        self.headers.header(name, value.as_bytes());
    }

    /// Start a test server at 127.0.0.1:7878.
    pub fn start_server() {
        if let Err(e) = utils::build_server() {
            eprintln!("{RED}Server failed to build: {e}{RESET}");
            process::exit(1);
        }

        let args = [
            "run", "--bin", "server", "--", "--test", "--", TEST_SERVER_ADDR
        ];

        if let Err(e) = Command::new("cargo")
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            eprintln!("{RED}Failed to start server: {e}{RESET}");
            process::exit(1);
        }

        if utils::check_server_is_live(TEST_SERVER_ADDR) {
            println!(
                "{GREEN}Server is listening on {TEST_SERVER_ADDR}.{RESET}"
            );
            process::exit(0);
        }

        eprintln!("{RED}Failed to start server.{RESET}");
        process::exit(1);
    }

    /// Shuts down a test server running at 127.0.0.1:7878.
    pub fn do_shutdown() {
        let uri = format!("{TEST_SERVER_ADDR}/");

        if let Err(e) = Client::send(Method::Shutdown, &uri) {
            eprintln!("Could not send the shutdown request.\n{e}");
        }

        process::exit(0);
    }
}

/// Contains the parsed server command line arguments.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerCli {
    pub do_log: bool,
    pub do_debug: bool,
    pub is_test: bool,
    pub addr: Option<String>,
    pub log_file: Option<PathBuf>,
    pub router: Router,
}

impl Default for ServerCli {
    fn default() -> Self {
        Self {
            do_log: false,
            do_debug: false,
            is_test: false,
            addr: None,
            log_file: None,
            router: Router::new()
        }
    }
}

impl Display for ServerCli {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:?}")
    }
}

impl Debug for ServerCli {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(
            f,
            "ServerCli {{\n    \
            do_log: {:?},\n    \
            do_debug: {:?},\n    \
            is_test: {:?},",
            self.do_log,
            self.do_debug,
            self.is_test
        )?;

        if let Some(addr) = self.addr.as_ref() {
            writeln!(f, "    addr: Some({addr:?}),")?;
        } else {
            writeln!(f, "    addr: None,")?;
        }

        if let Some(log_file) = self.log_file.as_ref() {
            writeln!(f, "    log_file: Some({:?}),", log_file.display())?;
        } else {
            writeln!(f, "    log_file: None,")?;
        }

        if self.router.is_empty() {
            writeln!(f, "    router: Router()")?;
        } else {
            writeln!(f, "    router: Router(")?;

            for route in &self.router.0 {
                writeln!(f, "        {route:?},")?;
            }

            writeln!(f, "    )")?;
        }

        write!(f, "}}")
    }
}

impl TryFrom<ServerCli> for Server {
    type Error = NetError;

    fn try_from(mut cli: ServerCli) -> NetResult<Self> {
        let Some(addr) = cli.addr.take() else {
            return Err(NetError::Other("Missing server address.".into()));
        };

        let mut server = Self::builder();

        if let Some(path) = cli.log_file.take() {
            let _ = server.log_file(path);
            cli.do_log = true;
        }

        server
            .addr(&addr)
            .do_log(cli.do_log)
            .do_debug(cli.do_debug)
            .is_test_server(cli.is_test)
            .router(&mut cli.router)
            .build()
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

        match opt {
            "-I" | "--favicon" => match tokens.next() {
                Some(file_path) => {
                    let _ = self.router.favicon(PathBuf::from(file_path));
                },
                None => self.invalid_arg(opt, arg),
            },
            "-0" | "--not-found" => match tokens.next() {
                Some(file_path) => {
                    let _ = self.router.not_found(PathBuf::from(file_path));
                },
                None => self.invalid_arg(opt, arg),
            },
            "-T" | "--text" | "-F" | "--file" => {
                let (Some(method), Some(uri_path), Some(target)) = (
                    tokens.next(),
                    tokens.next(),
                    tokens.next()
                ) else {
                    self.invalid_arg(opt, arg);
                    return;
                };

                let method = method.to_ascii_uppercase();

                let Ok(method) = Method::from_str(method.as_str()) else {
                    return self.invalid_arg(opt, arg);
                };

                let uri_path = uri_path.to_ascii_lowercase();

                let target: Target = match opt {
                    "-T" | "--text" => String::from(target).into(),
                    "-F" | "--file" => PathBuf::from(target).into(),
                    _ => unreachable!(),
                };

                let route = Route::new(method, uri_path.into(), target);
                self.router.mount(route);
            },
            _ => unreachable!(),
        }
    }

    /// Prints the server help message and exists the program.
    pub fn print_help(&self) {
        eprintln!("\
{GREEN}USAGE:{RESET}
    {SERVER_NAME} [OPTIONS] [ROUTES] [--] <SERVER ADDRESS>\n
{GREEN}SERVER ADDRESS:{RESET}
    IP:PORT              The server's IP address and port.\n
{GREEN}OPTIONS:{RESET}
    -d, --debug          Prints debug information.
    -h, --help           Prints this help message.
    -l, --log            Enables logging of connections to stdout.
    -f, --log-file FILE  Enables logging of connections to FILE.
    -t, --test           Creates a test server at {TEST_SERVER_ADDR}.\n
{GREEN}ROUTES:{RESET}
    -I, --favicon FILE_PATH
        Adds a route that serves a favicon.
    -0, --not-found FILE_PATH
        Adds a route that handles 404 Not Found responses.
    -T, --text METHOD:URI_PATH:TEXT
        Adds a route that serves text.
    -F, --file METHOD:URI_PATH:FILE_PATH
        Adds a route that serves a file.\n");

        process::exit(0);
    }

    /// Parses the command line arguments into a `ServerCli` object.
    #[must_use]
    pub fn parse_args(args: &mut VecDeque<&str>) -> Self {
        let mut cli = Self::new();

        let _ = args.pop_front();

        if args.is_empty() {
            cli.print_help();
            process::exit(0);
        }

        while let Some(opt) = args.pop_front() {
            match opt {
                // First argument after "--" is the server address.
                "--" => match args.pop_front() {
                    Some(arg) => {
                        cli.addr = Some(arg.to_string());
                        break;
                    },
                    None => cli.missing_arg("SERVER ADDRESS"),
                },
                // Handle options.
                _ if opt.starts_with('-') => match opt {
                    // Enable logging of new connections.
                    "-l" | "--log" => cli.do_log = true,
                    // Enable debug printing.
                    "-d" | "--debug" => cli.do_debug = true,
                    // Print help message.
                    "-h" | "--help" => cli.print_help(),
                    // Make the server a test server.
                    "-t" | "--test" => {
                        cli.is_test = true;
                        let  _ = cli.router.shutdown();
                    },
                    // Set a local log file.
                    "-f" | "--log-file" => match args.pop_front() {
                        Some(arg) => {
                            cli.do_log = true;
                            cli.log_file = Some(PathBuf::from(arg));
                        },
                        None => cli.missing_arg(opt),
                    },
                    // Add a route.
                    "-F" | "--file"
                        | "-I" | "--favicon"
                        | "-0" | "--not-found"
                        | "-T" | "--text" => match args.pop_front() {
                        Some(arg) => cli.parse_route(opt, arg),
                        None => cli.missing_arg(opt),
                    },
                    // Unknown option.
                    _ => cli.unknown_opt(opt),
                },
                // First non-option argument is the server address.
                _ => {
                    cli.addr = Some(opt.to_string());
                    break;
                },
            }
        }

        cli
    }
}

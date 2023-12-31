use std::collections::VecDeque;
use std::net::{SocketAddr, TcpStream};
use std::process::{self, Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crate::{
    Body, Client, Headers, Method, NetResult, Style, Tui, UriPath, Version,
    WriteCliError, TEST_SERVER_ADDR,
};
use crate::style::colors::{BR_GRN, BR_RED, CLR};
use crate::util;

/// Contains the parsed client command line arguments.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
{BR_GRN}USAGE:{CLR}
    http_client [OPTIONS] [--] <URI>\n
{BR_GRN}ARGUMENT:{CLR}
    URI   An HTTP URI (e.g. \"httpbin.org/json\")\n
{BR_GRN}OPTIONS:{CLR}
    -B, --body TEXT         Add TEXT to the request body.
    -b, --builder           Build a request and send it.
    -d, --debug             Print client debug information.
    -H, --header HEADER     Add a header with the format NAME:VALUE to the request.
    -h, --help              Display this help message.
    -M, --method METHOD     Use METHOD as the request method (default: \"GET\").
    -m, --minimal           Only print the request line and status line.
    -n, --no-dates          Remove Date headers from the output (useful during testing).
    -O, --output FORMAT     Set the output style to FORMAT, see below
                            (default: --output \"shb\").
    -P, --path PATH         Use PATH as the URI path (default: \"/\").
    -p, --plain             Do not colorize the output.
    -r, --request           Print the request without sending it.
    -s, --server            Start a server listening on {TEST_SERVER_ADDR}.
    -S, --shutdown          Shut down the server running on {TEST_SERVER_ADDR}.
    -T, --tui               Run the client TUI.
    -v, --verbose           Print both the request and the response.\n
    -V, --version           Set the protocol version (default: \"HTTP/1.1\").\n
{BR_GRN}FORMAT OPTIONS:{CLR}
    R = request line       s = status line
    H = request headers    h = response headers
    B = request body       b = response body\n");

        process::exit(0);
    }

    /// Parses the command line options into a `ClientCli` object.
    ///
    /// # Errors
    ///
    /// Returns an error if the `Client` cannot be built.
    pub fn parse_args(args: &mut VecDeque<&str>) -> NetResult<Self> {
        let _ = args.pop_front();

        let mut cli = Self::new();

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
                "--builder" => {
                    Client::get_request_from_cli()?;
                    break;
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

        Ok(cli)
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
                    self.path = path.into();
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
                Some(body) => {
                    let body = Vec::from(body.trim());
                    self.body = Body::Text(body.into());
                },
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
        match util::parse_uri(arg).ok() {
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

        let name = name.to_ascii_lowercase();
        let value = value.to_ascii_lowercase();

        let name = util::to_titlecase(name.as_bytes());

        self.headers.header(name.trim(), value.trim());
    }

    pub fn do_shutdown() {
        let uri = format!("{TEST_SERVER_ADDR}/");

        if let Err(e) = Client::send(Method::Shutdown, &uri) {
            eprintln!("Could not send the shutdown request.\n{e}");
        }

        process::exit(0);
    }

    pub fn start_server() {
        if let Err(e) = util::build_server() {
            eprintln!("{BR_RED}Server failed to build: {e}{CLR}");
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
            eprintln!("{BR_RED}Failed to start server: {e}{CLR}");
            process::exit(1);
        }

        if Self::check_server(TEST_SERVER_ADDR) {
            println!("{BR_GRN}Server is listening on {TEST_SERVER_ADDR}.{CLR}");
            process::exit(0);
        }

        eprintln!("{BR_RED}Failed to start server.{CLR}");
        process::exit(1);
    }

    /// Returns true if a connection can be established with the given `addr`.
    #[must_use]
    pub fn check_server(addr: &str) -> bool {
        let timeout = Duration::from_millis(200);

        let Ok(socket) = SocketAddr::from_str(addr) else {
            return false;
        };

        // Attempt to connect a maximum of five times.
        for _ in 0..5 {
            if TcpStream::connect_timeout(&socket, timeout).is_ok() {
                return true;
            }

            thread::sleep(timeout);
        }

        false
    }
}

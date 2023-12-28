use std::borrow::{Cow, Borrow};
use std::collections::VecDeque;
use std::net::{SocketAddr, TcpStream};
use std::process::{self, Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crate::{
    Body, Client, ClientBuilder, Headers, Method, NetError, NetResult,
    OutputStyle, Tui, WriteCliError,
};
use crate::colors::{CLR, GRN, RED};
use crate::config::TEST_SERVER_ADDR;
use crate::header_name::{CONTENT_LENGTH, CONTENT_TYPE};
use crate::util;

/// Contains the parsed client command line arguments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientCli<'a> {
    pub debug: bool,
    pub do_send: bool,
    pub method: Method,
    pub path: Option<Cow<'a, str>>,
    pub addr: Option<Cow<'a, str>>,
    pub headers: Headers,
    pub body: Body,
    pub output: OutputStyle,
}

impl<'a> Default for ClientCli<'a> {
    fn default() -> Self {
        Self {
            debug: false,
            do_send: true,
            method: Method::Get,
            path: None,
            addr: None,
            headers: Headers::new(),
            body: Body::Empty,
            output: OutputStyle::default()
        }
    }
}

impl<'a> WriteCliError for ClientCli<'a> {}

impl<'a> ClientCli<'a> {
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
    --body TEXT         Add TEXT to the request body.
    --builder           Build a request and send it.
    --debug             Print client debug information.
    --header HEADER     Add a header with the format NAME:VALUE to the request.
    --help              Display this help message.
    --method METHOD     Use METHOD as the request method (default: \"GET\").
    --minimal           Only print the request line and status line.
    --no-dates          Remove Date headers from the output (useful during testing).
    --output FORMAT     Set the output style to FORMAT, see below
                        (default: --output \"shb\").
    --path PATH         Use PATH as the URI path (default: \"/\").
    --plain             Do not colorize the output.
    --request           Print the request without sending it.
    --server            Start a server listening on {TEST_SERVER_ADDR}.
    --shutdown          Shut down the server running on {TEST_SERVER_ADDR}.
    --tui               Run the client TUI.
    --verbose           Print both the request and the response.\n
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
    pub fn parse_args(args: &mut VecDeque<&str>) -> NetResult<Client> {
        let mut cli = Self::new();

        let _ = args.pop_front();

        while let Some(opt) = args.pop_front() {
            if !opt.starts_with("--") {
                // First non-option argument is the URI argument.
                cli.handle_uri(opt);
                break;
            }

            match opt.len() {
                2 if opt == "--" => match args.pop_front() {
                    // URI following the end of options flag.
                    Some(uri) => cli.handle_uri(uri),
                    None => cli.missing_arg("URI"),
                },
                // Run the client TUI.
                5 if opt == "--tui" => Tui::run(),
                6 => match opt {
                    // Print the help message.
                    "--help" => cli.print_help(),
                    // Set request body data.
                    "--body" => match args.pop_front() {
                        Some(text) => cli.handle_body(text),
                        None => cli.missing_arg(opt),
                    },
                    // Path component of the requested HTTP URI.
                    "--path" => match args.pop_front() {
                        Some(path) => cli.path = Some(path.into()),
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
                    // Start a test server at localhost:7878.
                    "--server" => ClientCli::start_server(),
                    // Set request method.
                    "--method" => match args.pop_front() {
                        Some(method) => cli.handle_method(method),
                        None => cli.missing_arg(opt),
                    },
                    // Add a request header.
                    "--header" => match args.pop_front() {
                        Some(header) => cli.handle_header(header),
                        None => cli.missing_arg(opt),
                    },
                    // Set the output style based on the format string arg.
                    "--output" => match args.pop_front() {
                        Some(format) => cli.output.format_str(format),
                        None => cli.missing_arg(opt),
                    },
                    _ => cli.unknown_opt(opt),
                },
                9 => match opt {
                    // Only print the request line and status line.
                    "--minimal" => cli.output.format_str("Rs"),
                    // Set verbose output style.
                    "--verbose" => cli.output.format_str("RHBshb"),
                    // Run the request builder.
                    "--builder" => return cli.build_request(),
                    // Set request output style and no send option.
                    "--request" => {
                        cli.do_send = false;
                        cli.output.format_str("RHB");
                    },
                    _ => cli.unknown_opt(opt),
                },
                10 => match opt {
                    // Remove Date headers before printing.
                    "--no-dates" => cli.output.no_dates = true,
                    // Send a shutdown request to localhost:7878.
                    "--shutdown" => cli.do_shutdown(),
                    _ => cli.unknown_opt(opt),
                },
                // Handle an unknown option.
                _ => cli.unknown_opt(opt),
            }
        }

        let Some(addr) = cli.addr.take() else {
            cli.missing_arg("URI");
            process::exit(1);
        };

        ClientBuilder::<&str>::new()
            .debug(cli.debug)
            .do_send(cli.do_send)
            .addr(addr.borrow())
            .path(cli.path.unwrap_or(Cow::Borrowed("/")).borrow())
            .method(cli.method.clone())
            .headers(cli.headers.clone())
            .output(cli.output)
            .body(cli.body)
            .build()
    }

    fn handle_method(&mut self, method: &str) {
        let method = method.to_ascii_uppercase();
        self.method = Method::from(method.as_str());
    }

    fn handle_header(&mut self, header: &str) {
        match header.split_once(':') {
            Some((name, value)) => {
                self.headers.header(name.trim(), value.trim());
            },
            None => self.invalid_arg("--header", header),
        }
    }

    fn handle_body(&mut self, body: &str) {
        self.body = Body::Text(Vec::from(body));

        if !self.headers.contains(&CONTENT_TYPE) {
            if let Some(con_type) = self.body.as_content_type() {
                self.headers.content_type(con_type);
            }
        }

        if !self.headers.contains(&CONTENT_LENGTH) {
            self.headers.content_length(self.body.len());
        }
    }

    fn handle_uri(&mut self, uri: &str) {
        match util::parse_uri(uri).ok() {
            Some((addr, path)) if self.path.is_none() => {
                self.addr = Some(addr.into());
                self.path = Some(path.into());
            },
            // Do not clobber a previously set path.
            Some((addr, _)) => self.addr = Some(addr.into()),
            None => self.invalid_arg("URI", uri),
        }
    }

    fn do_shutdown(&self) {
        if let Err(e) = Client::shutdown(TEST_SERVER_ADDR) {
            eprintln!("Could not send the shutdown request.\n{e}");
        }

        process::exit(0);
    }

    fn start_server() {
        if let Err(e) = util::build_server() {
            eprintln!("{RED}Server failed to build: {e}{CLR}");
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
            eprintln!("{RED}Failed to start server: {e}{CLR}");
            process::exit(1);
        }

        if Self::check_server().is_ok() {
            println!("{GRN}Server is listening on {TEST_SERVER_ADDR}.{CLR}");
            process::exit(0);
        }

        eprintln!("{RED}Failed to start server.{CLR}");
        process::exit(1);
    }

    fn check_server() -> NetResult<()> {
        let timeout = Duration::from_millis(200);

        let Ok(socket) = SocketAddr::from_str(TEST_SERVER_ADDR) else {
            return Err(NetError::NotConnected);
        };

        // Attempt to connect a maximum of five times.
        for _ in 0..5 {
            if TcpStream::connect_timeout(&socket, timeout).is_ok() {
                return Ok(());
            }

            thread::sleep(timeout);
        }

        Err(NetError::NotConnected)
    }

    pub fn build_request(&self) -> NetResult<Client> {
        let (mut req, conn) = Tui::build_request()?;

        let remote_addr = conn.remote_addr.to_string();
        req.headers.header("Host", &remote_addr);

        let client = Client {
            debug: false,
            do_send: true,
            req: Some(req),
            res: None,
            conn: Some(conn),
            output: OutputStyle::default()
        };

        Ok(client)
    }
}

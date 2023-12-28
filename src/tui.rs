#![allow(clippy::missing_errors_doc)]

use std::io::{self, BufRead, BufWriter, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{self, Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use std::str::FromStr;

use crate::{
    Body, Client, Connection, HeaderValue, Method, NetError, NetParseError,
    NetResult, Request, RequestBuilder,
};
use crate::colors::{BLU, CLR, CYAN, GRN, PURP, RED, YLW};
use crate::config::TEST_SERVER_ADDR;
use crate::header_name::{CONNECTION, HOST};
use crate::util;

/// A shell-like TUI for an HTTP client.
#[derive(Debug)]
pub struct Tui {
    pub do_log: bool,
    pub do_send: bool,
    pub running: bool,
    pub last_addr: Option<String>,
    pub last_code: Option<u16>,
    pub client: Client,
    pub server: Option<Child>,
}

impl Default for Tui {
    fn default() -> Self {
        Self {
            do_log: false,
            do_send: true,
            running: true,
            last_addr: None,
            last_code: None,
            client: Client::default(),
            server: None
        }
    }
}

impl Tui {
    /// Returns a new `Tui` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the client TUI.
    pub fn run() {
        if let Err(e) = Self::run_main_loop() {
            eprintln!("{RED}Error: {e}{CLR}");
            process::exit(1);
        }

        process::exit(0);
    }

    /// Runs the main IO loop.
    pub fn run_main_loop() -> NetResult<()> {
        let mut tui = Self::new();
        let mut line = String::with_capacity(1024);

        Self::print_intro()?;

        while tui.running {
            tui.print_prompt()?;

            line.clear();
            io::stdin().lock().read_line(&mut line)?;

            if let Err(e) = tui.handle_user_input(line.trim()) {
                eprintln!("{RED}{e}{CLR}");
            }
        }

        // Ensure test server is closed.
        tui.kill_server(true)?;

        // Print newline on exit.
        println!();
        Ok(())
    }

    /// Parses user input into a command or URI and handles execution of the
    /// next steps.
    pub fn handle_user_input(&mut self, input: &str) -> NetResult<()> {
        match input {
            "" => {},
            "help" => Self::print_help(),
            "quit" => self.running = false,
            "clear" => Self::clear_screen()?,
            "body"
                | "minimal"
                | "request"
                | "response"
                | "verbose" => self.output_style(input),
            "builder" => {
                let (mut req, conn) = Self::build_request()?;

                let remote_addr = conn.remote_addr.to_string();
                req.headers.header("Host", &remote_addr);

                self.client.req = Some(req);
                self.client.conn = Some(conn);
                self.last_addr = Some(remote_addr);

                println!();
                self.send_request_and_print_output()?;
            },
            "log-server" => self.toggle_logging(),
            "start-server" => {
                if self.server.is_some() {
                    eprintln!(
                        "{YLW}Server is already running.\nPlease run \
                        `kill-server` to shut that server down first before \
                        starting\na new one.{CLR}\n"
                    );
                }

                let server = Self::start_server()?;
                self.server = Some(server);
                self.last_addr = Some(TEST_SERVER_ADDR.to_string());
            },
            "kill-server" => self.kill_server(false)?,
            _ => {
                if self.parse_input(input).is_err() {
                    return Ok(());
                }

                if self.do_send {
                    self.send_request_and_print_output()?;
                } else if let Some(req) = self.client.req.as_mut() {
                    // Update request headers for the "request" output style.
                    if !req.headers.contains(&HOST) {
                        if let Some(conn) = self.client.conn.as_mut() {
                            let stream = conn.writer.get_ref();

                            if let Ok(remote) = stream.peer_addr() {
                                req.headers.host(remote);
                            }
                        }
                    }

                    self.print_output()?;
                }
            },
        }

        Ok(())
    }

    /// Parses an input string into a request and adds it to the client.
    pub fn parse_input(&mut self, input: &str) -> NetResult<()> {
        let mut req = RequestBuilder::new();

        let uri = match input.split_once(' ') {
            None => input,
            Some((method, uri)) => {
                let method = method.to_ascii_uppercase();
                req.method(Method::from(method.as_str()));
                uri
            },
        };

        match util::parse_uri(uri).ok() {
            Some((addr, path)) => {
                let Ok(conn) = Connection::try_from(addr.as_str())
                else {
                    self.warn_no_connection(&addr)?;
                    return Err(NetError::NotConnected);
                };

                req.path(&path);
                self.last_addr = Some(addr);
                self.client.conn = Some(conn);
                self.client.req = Some(req.build());
                Ok(())
            },
            None if uri.starts_with('/') && self.last_addr.is_some() => {
                let Some(addr) = self.last_addr.as_ref()
                else {
                    Self::warn_invalid_input(uri);
                    return Err(NetParseError::Path.into());
                };

                // Need clone due to upcoming mutable borrow of self.
                let addr = addr.clone();

                let Ok(conn) = Connection::try_from(addr.as_str())
                else {
                    self.warn_no_connection(&addr)?;
                    return Err(NetError::NotConnected);
                };

                req.path(uri);
                self.client.conn = Some(conn);
                self.client.req = Some(req.build());
                Ok(())
            },
            None => {
                Self::warn_invalid_input(uri);
                Err(NetParseError::Path)?
            },
        }
    }

    /// Sets the output style and prints a message to stdout.
    pub fn output_style(&mut self, style: &str) {
        // Reset the do_send option on style change.
        self.do_send = true;

        match style {
            "body" => self.client.output.format_str("b"),
            "response" => self.client.output.format_str("shb"),
            "minimal" => self.client.output.format_str("Rs"),
            "verbose" => self.client.output.format_str("*"),
            "request" => {
                self.do_send = false;
                self.client.output.format_str("RHB");
            },
            _ => unreachable!(),
        }

        println!("Output set to {PURP}{style}{CLR} style.\n");
    }

    /// Clears the screen and moves the cursor to the top left.
    pub fn clear_screen() -> NetResult<()> {
        let mut stdout = io::stdout().lock();

        // Clear screen.
        stdout.write_all(b"\x1b[2J")?;

        // Move cursor to top left.
        stdout.write_all(b"\x1b[1;1H")?;

        stdout.flush()?;
        Ok(())
    }

    /// Prints the intro message on program start-up.
    pub fn print_intro() -> NetResult<()> {
        let face = format!(r#"
              .-''''''-.
            .' _      _ '.
           /   {CYAN}O      {CYAN}O{PURP}   \
          :                :
          |                |
          :       __       :
           \  .-"`  `"-.  /
            '.          .'
              '-......-'

         YOU SHOULDN'T BE HERE"#);


        Self::clear_screen()?;
        println!("{PURP}http_tui/0.1\n\n{face}{CLR}\n");
        Ok(())
    }

    /// Prints the prompt.
    pub fn print_prompt(&mut self) -> NetResult<()> {
        let mut stdout = io::stdout().lock();

        match self.last_code.take() {
            None => write!(&mut stdout, "{CYAN}${CLR} ")?,
            Some(code) => {
                let color = match code {
                    100..=199 | 300..=399 => BLU,
                    200..=299 => GRN,
                    400..=599 => RED,
                    _ => YLW,
                };

                write!(
                    &mut stdout,
                    "{CYAN}[{color}{code}{CYAN}]${CLR} "
                )?;
            },
        }

        stdout.flush()?;
        Ok(())
    }

    /// Prints the help message to stdout.
    pub fn print_help() {
        eprintln!("\
{PURP}http_tui{CLR} is a shell-like HTTP client.\n
Enter a `{PURP}URI{CLR}` to send a GET request.
    e.g. \"httpbin.org/status/201\"\n
To send a request with a different method enter `{PURP}METHOD URI{CLR}`.
    e.g. \"POST httpbin.org/status/201\"\n
Additional requests to the same address can be entered `{PURP}/PATH{CLR}`.
    e.g. \"/status/201\"\n
The prior response's status code is displayed in the prompt.\n
{PURP}COMMANDS:{CLR}
    body          Only print the response bodies.
    builder       Build a request and send it.
    clear         Clear the screen.
    help          Print this help message.
    kill-server   Shut down the test server.
    minimal       Only print the request lines and status lines.
    quit          Quit the program.
    request       Only print the requests (but do not send them).
    response      Only print the responses (default).
    start-server  Start a test server at localhost:7878.
    verbose       Print both requests and responses.\n"
        );
    }

    /// Prints a warning to stdout that `input` was invalid.
    pub fn warn_invalid_input(input: &str) {
        eprintln!("{RED}Invalid input: \"{input}\"{CLR}\n");
    }

    /// Prints a warning to stdout that connecting to `addr` failed.
    pub fn warn_no_connection(&mut self, addr: &str) -> NetResult<()> {
        // Reset prior connection.
        self.last_addr = None;
        self.client.req = None;
        self.client.res = None;
        self.client.conn = None;

        println!("{RED}Unable to connect to \"{addr}\"{CLR}\n");
        Ok(())
    }

    /// Starts a test server at 127.0.0.1:7878.
    pub fn start_server() -> NetResult<Child> {
        if let Err(e) = util::build_server() {
            eprintln!("{RED}Server failed to build: {e}{CLR}\n");
            return Err(NetError::NotConnected);
        }

        let args = [
            "run", "--bin", "server", "--", "--test", "--", TEST_SERVER_ADDR
        ];

        let server = match Command::new("cargo")
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(server) => server,
            Err(e) => {
                eprintln!("{RED}Failed to start: {e}{CLR}\n");
                return Err(NetError::NotConnected);
            },
        };

        match Self::check_server() {
            Ok(()) => {
                println!(
                    "{GRN}Server is running on {TEST_SERVER_ADDR}.{CLR}\n"
                );

                Ok(server)
            },
            Err(e) => {
                eprintln!("{RED}Failed to start server.\n{e}{CLR}\n");
                Err(NetError::NotConnected)
            },
        }
    }

    /// Shuts the test server down.
    pub fn kill_server(&mut self, quiet: bool) -> NetResult<()> {
        let Some(mut server) = self.server.take() else {
            if !quiet {
                eprintln!("{YLW}No active server found.{CLR}\n");
            }

            return Ok(());
        };

        Client::shutdown(TEST_SERVER_ADDR)?;

        match server.kill() {
            Ok(()) if !quiet => {
                println!("{GRN}Server has been shut down.{CLR}\n");
            },
            Err(e) if !quiet => {
                eprintln!("{RED}Unable to shut down server.\n{e}{CLR}\n");
            },
            _ => {},
        }

        self.last_addr = None;
        Ok(())
    }

    /// Ensures that the server is live.
    pub fn check_server() -> NetResult<()> {
        let socket = SocketAddr::from_str(TEST_SERVER_ADDR)
            .map_err(|_| NetError::NotConnected)?;

        let timeout = Duration::from_millis(200);

        // Attempt to connect a maximum of five times.
        for _ in 0..5 {
            if TcpStream::connect_timeout(&socket, timeout).is_ok() {
                return Ok(());
            }

            thread::sleep(timeout);
        }

        Err(NetError::NotConnected)
    }

    /// Toggles printing of server log messages to stdout.
    pub fn toggle_logging(&mut self) {
        self.do_log = !self.do_log;

        if self.do_log {
            println!("Server logging {PURP}enabled{CLR}.\n");
        } else {
            println!("Server logging {PURP}disabled{CLR}.\n");
        }
    }

    /// Returns true if the response contains a Connection header with the
    /// value "close".
    #[must_use]
    pub fn connection_is_closed(&self) -> bool {
        self.client
            .res
            .as_ref()
            .map_or(false, |res| {
                let value = res.headers.get(&CONNECTION);
                value == Some(&HeaderValue::from("close"))
            })
    }

    /// Sends a request, receives a response, and handles printing the output
    // to stdout.
    pub fn send_request_and_print_output(&mut self) -> NetResult<()> {
        if let Err(e) = self.client.send_request() {
            let msg = format!(
                "{RED}Error while sending request:\n{}{CLR}\n",
                e.to_string().trim_end()
            );

            self.client.conn = None;
            return Err(NetError::Other(msg.into()));
        }

        if let Err(e) = self.client.recv_response() {
            let msg = format!(
                "{RED}Error while receiving response:\n{}{CLR}\n",
                e.to_string().trim_end()
            );

            self.client.conn = None;
            return Err(NetError::Other(msg.into()));
        }

        if self.connection_is_closed() {
            self.client.conn = None;
        }

        self.print_output()?;
        Ok(())
    }

    /// Handles printing output to stdout.
    pub fn print_output(&mut self) -> NetResult<()> {
        let mut stdout = BufWriter::new(io::stdout().lock());
        self.client.print(&mut stdout)?;

        // Store status code for prompt.
        if let Some(res) = self.client.res.as_ref() {
            self.last_code = Some(res.status_code());
        }

        // Remove request and response after printing.
        self.client.req = None;
        self.client.res = None;
        Ok(())
    }

    /// Reads and parses a method from stdin.
    pub fn get_method_from_user(
        line: &mut String,
        req: &mut RequestBuilder
    ) -> NetResult<()> {
        let mut stdin = io::stdin().lock();
        let mut stdout = io::stdout().lock();

        write!(&mut stdout, "{CYAN}Method [GET]:{CLR} ")?;
        stdout.flush()?;

        line.clear();
        stdin.read_line(line)?;

        if line.trim().is_empty() {
            req.method(Method::Get);
        } else {
            let uppercase = line.trim().to_ascii_uppercase();
            req.method(Method::from(uppercase.as_str()));
        }

        Ok(())
    }

    /// Reads and parses an address from stdin and then connects to it.
    pub fn get_addr_from_user(line: &mut String) -> NetResult<Connection> {
        let mut stdin = io::stdin().lock();
        let mut stdout = io::stdout().lock();

        loop {
            write!(&mut stdout, "{CYAN}Address:{CLR} ")?;
            stdout.flush()?;

            line.clear();
            stdin.read_line(line)?;

            if line.trim().is_empty() {
                continue;
            }

            let addr = if line.contains(':') {
                line.trim().to_string()
            } else {
                format!("{}:80", line.trim())
            };

            match Connection::try_from(addr.as_str()) {
                Ok(conn) => return Ok(conn),
                Err(e) => {
                    writeln!(
                        &mut stdout,
                        "{RED}Unable to connect to \"{}\".\n{e}{CLR}",
                        line.trim()
                    )?;

                    stdout.flush()?;
                },
            }
        }
    }

    /// Reads and parses a path from stdin.
    pub fn get_path_from_user(
        line: &mut String,
        req: &mut RequestBuilder
    ) -> NetResult<()> {
        let mut stdin = io::stdin().lock();
        let mut stdout = io::stdout().lock();

        loop {
            write!(&mut stdout, "{CYAN}Path:{CLR} ")?;
            stdout.flush()?;

            line.clear();
            stdin.read_line(line)?;

            if line.starts_with('/') {
                req.path(line.trim());
                return Ok(());
            }

            writeln!(
                &mut stdout,
                "{RED}Invalid input. Paths start with a \"/\".{CLR}"
            )?;
        }
    }

    /// Reads and parses headers from stdin.
    pub fn get_headers_from_user(
        line: &mut String,
        req: &mut RequestBuilder
    ) -> NetResult<()> {
        let mut stdin = io::stdin().lock();
        let mut stdout = io::stdout().lock();

        loop {
            write!(
                &mut stdout,
                "{CYAN}Header (optional; \"name:value\"):{CLR} "
            )?;
            stdout.flush()?;

            line.clear();
            stdin.read_line(line)?;

            if line.trim().is_empty() {
                return Ok(());
            }

            if let Some((name, value)) = line.trim().split_once(':') {
                req.header(name, value);
                continue;
            }

            writeln!(
                &mut stdout,
                "{RED}Invalid input.\n\
                The header name and value should be separated with \
                a \":\".{CLR}"
            )?;
        }
    }

    /// Reads and parses a body from stdin.
    pub fn get_body_from_user(
        line: &mut String,
        req: &mut RequestBuilder
    ) -> NetResult<()> {
        let mut stdin = io::stdin().lock();
        let mut stdout = io::stdout().lock();

        write!(&mut stdout, "{CYAN}Body (optional):{CLR} ")?;
        stdout.flush()?;

        line.clear();
        stdin.read_line(line)?;

        if line.trim().is_empty() {
            req.body(Body::Empty);
        } else {
            req.body(Body::Text(Vec::from(line.trim())));
        }

        Ok(())
    }

    /// Builds a user-defined request and returns it.
    pub fn build_request() -> NetResult<(Request, Connection)> {
        let mut req = RequestBuilder::new();
        let mut line = String::with_capacity(1024);

        Self::get_method_from_user(&mut line, &mut req)?;
        let conn = Self::get_addr_from_user(&mut line)?;
        Self::get_path_from_user(&mut line, &mut req)?;
        Self::get_headers_from_user(&mut line, &mut req)?;
        Self::get_body_from_user(&mut line, &mut req)?;

        let req = req.build();
        Ok((req, conn))
    }
}

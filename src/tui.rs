#![allow(clippy::missing_errors_doc)]

use std::io::{self, BufRead, Write};
use std::process::{self, Child, Command, Stdio};
use std::str::FromStr;

use crate::{
    Client, Connection, HeaderValue, Method, NetError, NetParseError,
    NetResult, Request, TEST_SERVER_ADDR,
};
use crate::header::names::{CONNECTION, HOST};
use crate::style::colors::{
    BR_BLU, BR_CYAN, BR_GRN, BR_PURP, BR_RED, BR_YLW, CLR,
};
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
            eprintln!("{BR_RED}Error: {e}{CLR}");
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
                eprintln!("{BR_RED}{e}{CLR}");
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
                let cli = Client::get_request_from_cli()?;
                self.client = Client::try_from(cli)?;

                if let Some(conn) = self.client.conn.as_ref() {
                    self.last_addr = Some(conn.remote_addr.to_string());
                }

                println!();
                self.send_request_and_print_output()?;
            },
            "log-server" => self.toggle_logging(),
            "start-server" => {
                if self.server.is_some() {
                    eprintln!(
                        "{BR_YLW}Server is already running.\nPlease run \
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

                    self.print_output();
                }
            },
        }

        Ok(())
    }

    /// Parses an input string into a request and adds it to the client.
    pub fn parse_input(&mut self, input: &str) -> NetResult<()> {
        let mut builder = Request::builder();

        let uri = match input.split_once(' ') {
            None => input,
            Some((method, uri)) => {
                let method = method.to_ascii_uppercase();
                let method = Method::from_str(method.as_str())?;
                builder.method(method);
                uri
            },
        };

        match util::parse_uri(uri).ok() {
            Some((addr, path)) => {
                let req = builder.path(path.into()).build();
                self.client.req = Some(req);
                self.client.conn = Some(Connection::try_from(addr.as_str())?);
                self.last_addr = Some(addr);
                Ok(())
            },
            None if uri.starts_with('/') && self.last_addr.is_some() => {
                let Some(addr) = self.last_addr.as_ref() else {
                    Self::warn_invalid_input(uri);
                    return Err(NetParseError::Path.into());
                };

                let req = builder.path(uri.into()).build();
                self.client.req = Some(req);
                self.client.conn = Some(Connection::try_from(addr.as_str())?);
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
            "body" => self.client.style.from_format_str("b"),
            "response" => self.client.style.from_format_str("shb"),
            "minimal" => self.client.style.from_format_str("Rs"),
            "verbose" => self.client.style.from_format_str("*"),
            "request" => {
                self.do_send = false;
                self.client.style.from_format_str("RHB");
            },
            _ => unreachable!(),
        }

        println!("Output style set to: {BR_PURP}{style}{CLR}\n");
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
           /   {BR_CYAN}O      {BR_CYAN}O{BR_PURP}   \
          :                :
          |                |
          :       __       :
           \  .-"`  `"-.  /
            '.          .'
              '-......-'

         YOU SHOULDN'T BE HERE"#);


        Self::clear_screen()?;
        println!("{BR_PURP}http_tui/0.1\n\n{face}{CLR}\n");
        Ok(())
    }

    /// Prints the prompt.
    pub fn print_prompt(&mut self) -> NetResult<()> {
        let mut stdout = io::stdout().lock();

        match self.last_code.take() {
            None => write!(&mut stdout, "{BR_CYAN}${CLR} ")?,
            Some(code) => {
                let color = match code {
                    100..=199 | 300..=399 => BR_BLU,
                    200..=299 => BR_GRN,
                    400..=599 => BR_RED,
                    _ => BR_YLW,
                };

                write!(
                    &mut stdout,
                    "{BR_CYAN}[{color}{code}{BR_CYAN}]${CLR} "
                )?;
            },
        }

        stdout.flush()?;
        Ok(())
    }

    /// Prints the help message to stdout.
    pub fn print_help() {
        eprintln!("\
{BR_PURP}http_tui{CLR} is a shell-like HTTP client.\n
Enter a {BR_PURP}URI{CLR} to send a GET request.
    e.g. \"httpbin.org/status/201\"\n
To send a request with a different method enter {BR_PURP}METHOD URI{CLR}.
    e.g. \"POST httpbin.org/status/201\"\n
Additional requests to the same address can be entered {BR_PURP}/PATH{CLR}.
    e.g. \"/status/201\"\n
The prior response's status code is displayed in the prompt.\n
{BR_PURP}COMMANDS:{CLR}
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
        eprintln!("{BR_RED}Invalid input: \"{input}\"{CLR}\n");
    }

    /// Prints a warning to stdout that connecting to `addr` failed.
    pub fn warn_no_connection(&mut self, addr: &str) -> NetResult<()> {
        // Reset prior connection.
        self.last_addr = None;
        self.client.req = None;
        self.client.res = None;
        self.client.conn = None;

        println!("{BR_RED}Unable to connect to \"{addr}\"{CLR}\n");
        Ok(())
    }

    /// Starts a test server at 127.0.0.1:7878.
    pub fn start_server() -> NetResult<Child> {
        if let Err(e) = util::build_server() {
            eprintln!("{BR_RED}Server failed to build: {e}{CLR}\n");
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
                eprintln!("{BR_RED}Failed to start: {e}{CLR}\n");
                return Err(NetError::NotConnected);
            },
        };

        if util::check_server(TEST_SERVER_ADDR) {
            println!(
                "{BR_GRN}Server is listening at: {TEST_SERVER_ADDR}{CLR}\n"
            );

            Ok(server)
        } else {
            eprintln!("{BR_RED}Failed to start the server.{CLR}\n");
            Err(NetError::NotConnected)
        }
    }

    /// Shuts the test server down.
    pub fn kill_server(&mut self, quiet: bool) -> NetResult<()> {
        let Some(mut server) = self.server.take() else {
            if !quiet {
                eprintln!("{BR_YLW}No active server found.{CLR}\n");
            }

            return Ok(());
        };

        Client::send(Method::Shutdown, TEST_SERVER_ADDR)?;

        match server.kill() {
            Ok(()) if !quiet => {
                println!("{BR_GRN}Server has been shut down.{CLR}\n");
            },
            Err(e) if !quiet => {
                eprintln!("{BR_RED}Unable to shut down server.\n{e}{CLR}\n");
            },
            _ => {},
        }

        self.last_addr = None;
        Ok(())
    }

    /// Toggles printing of server log messages to stdout.
    pub fn toggle_logging(&mut self) {
        self.do_log = !self.do_log;

        if self.do_log {
            println!("Server logging {BR_PURP}enabled{CLR}.\n");
        } else {
            println!("Server logging {BR_PURP}disabled{CLR}.\n");
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
                "{BR_RED}Error while sending request:\n{}{CLR}\n",
                e.to_string().trim_end()
            );

            self.client.conn = None;
            return Err(NetError::Other(msg.into()));
        }

        if let Err(e) = self.client.recv_response() {
            let msg = format!(
                "{BR_RED}Error while receiving response:\n{}{CLR}\n",
                e.to_string().trim_end()
            );

            self.client.conn = None;
            return Err(NetError::Other(msg.into()));
        }

        if self.connection_is_closed() {
            self.client.conn = None;
        }

        self.print_output();
        Ok(())
    }

    /// Handles printing output to stdout.
    pub fn print_output(&mut self) {
        self.client.print();

        // Store status code for prompt.
        if let Some(res) = self.client.res.as_ref() {
            self.last_code = Some(res.status_code());
        }

        // Remove request and response after printing.
        self.client.req = None;
        self.client.res = None;
    }
}

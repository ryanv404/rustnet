use std::io::{self, BufRead, Write};
use std::process::{self, Child, Command, Stdio};
use std::str::FromStr;

use crate::{
    Client, Connection, HeaderValue, Method, NetError, NetParseError,
    NetResult, Request, TEST_SERVER_ADDR, TUI_NAME, utils,
};
use crate::headers::names::{CONNECTION, HOST};
use crate::style::colors::{
    BLUE, CYAN, GREEN, MAGENTA, ORANGE, RED, YELLOW, RESET,
};

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
            eprintln!("{RED}{e}{RESET}");
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
            line.clear();
            tui.print_prompt()?;
            io::stdin().lock().read_line(&mut line)?;

            if let Err(e) = tui.handle_user_input(line.trim()) {
                eprintln!("{RED}{e}{RESET}");
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
                self.client.get_request_from_user()?;

                // Set output style to "verbose".
                let old_style = self.client.style;
                self.client.style.from_format_str("*");

                if let Some(conn) = self.client.conn.as_ref() {
                    self.last_addr = Some(conn.remote_addr.to_string());
                }

                println!();
                self.send_request_and_print_output()?;

                // Restore old output style.
                self.client.style = old_style;
            },
            "start-server" => {
                if self.server.is_some() {
                    eprintln!(
                        "{YELLOW}Server is already running.\n\
                        Please run `kill-server` before starting a new test \
                        server.{RESET}\n"
                    );
                }

                self.server = Self::start_server().ok();
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
                    // Ensure the Host header is set.
                    if !req.headers.contains(&HOST) {
                        if let Some(conn) = self.client.conn.as_mut() {
                            let stream = conn.writer.get_ref();

                            if let Ok(addr) = stream.peer_addr() {
                                let addr = addr.to_string();
                                req.headers.insert(HOST, addr.as_str().into());
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
            Some((method, uri)) => {
                let method = method.to_ascii_uppercase();
                let method = Method::from_str(method.as_str())?;
                builder.method(method);
                uri
            },
            None => input,
        };

        match utils::parse_uri(uri).ok() {
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

                let req = builder.path(uri.to_string().into()).build();
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

        println!("Output style set to {MAGENTA}{style}{RESET}.\n");
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
        let version = env!("CARGO_PKG_VERSION");

        let face = format!(r#"
              .-''''''-.
            .' _      _ '.
           /   {CYAN}O      O{MAGENTA}   \
          :                :
          |                |
          :       __       :
           \  .-"`  `"-.  /
            '.          .'
              '-......-'

         YOU SHOULDN'T BE HERE"#);

        Self::clear_screen()?;

        println!("{MAGENTA}{TUI_NAME}\n{version}\n\n{face}{RESET}\n");

        Ok(())
    }

    /// Prints the prompt.
    pub fn print_prompt(&mut self) -> NetResult<()> {
        let mut stdout = io::stdout().lock();

        match self.last_code.take() {
            None => write!(&mut stdout, "{CYAN}${RESET} ")?,
            Some(code) => {
                let color = match code {
                    100..=199 => BLUE,
                    200..=299 => GREEN,
                    300..=399 => YELLOW,
                    400..=499 => ORANGE,
                    500..=599 => RED,
                    _ => MAGENTA,
                };

                write!(
                    &mut stdout,
                    "{CYAN}[{color}{code}{CYAN}]${RESET} "
                )?;
            },
        }

        stdout.flush()?;
        Ok(())
    }

    /// Prints the help message to stdout.
    pub fn print_help() {
        eprintln!("\
{MAGENTA}{TUI_NAME}{RESET} is a shell-like HTTP client.\n
Enter a {MAGENTA}URI{RESET} to send a GET request.
    e.g. \"httpbin.org/status/201\"\n
To send a request with a different method enter {MAGENTA}METHOD URI{RESET}.
    e.g. \"POST httpbin.org/status/201\"\n
Additional requests to the same address can be entered {MAGENTA}/PATH{RESET}.
    e.g. \"/status/201\"\n
{MAGENTA}COMMANDS:{RESET}
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
        eprintln!("{RED}Invalid input: \"{input}\"{RESET}\n");
    }

    /// Starts a test server at 127.0.0.1:7878.
    pub fn start_server() -> NetResult<Child> {
        if let Err(e) = utils::build_server() {
            eprintln!("{RED}Unable to build the server.\n{e}{RESET}\n");
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
                eprintln!("{RED}Unable to start the server.\n{e}{RESET}\n");
                return Err(NetError::NotConnected);
            },
        };

        if utils::check_server_is_live(TEST_SERVER_ADDR) {
            println!(
                "{GREEN}Server is listening at {TEST_SERVER_ADDR}{RESET}\n"
            );
            Ok(server)
        } else {
            eprintln!("{RED}Failed to start the server.{RESET}\n");
            Err(NetError::NotConnected)
        }
    }

    /// Shuts the test server down.
    pub fn kill_server(&mut self, quiet: bool) -> NetResult<()> {
        let Some(mut server) = self.server.take() else {
            if !quiet {
                eprintln!("{YELLOW}No active server found.{RESET}\n");
            }

            return Ok(());
        };

        Client::send(Method::Shutdown, TEST_SERVER_ADDR)?;

        match server.kill() {
            Ok(()) if !quiet => {
                println!("{GREEN}Server was shut down successfully.{RESET}\n");
            },
            Err(e) if !quiet => {
                eprintln!(
                    "{RED}Unable to shut down the server.\n{e}{RESET}\n"
                );
            },
            _ => {},
        }

        self.last_addr = None;
        Ok(())
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
                "{RED}Error while sending the request.\n\
                {}{RESET}\n",
                e.to_string().trim_end()
            );

            self.client.conn = None;
            return Err(NetError::Other(msg.into()));
        }

        if let Err(e) = self.client.recv_response() {
            let msg = format!(
                "{RED}Error while receiving the response.\n\
                {}{RESET}\n",
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
            self.last_code = Some(res.status.code());
        }

        // Remove request and response after printing.
        self.client.req = None;
        self.client.res = None;
    }
}

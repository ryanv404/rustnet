use std::io::{self, BufRead, BufWriter, StdoutLock, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{self, Child, Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crate::{
    Client, Connection, HeaderValue, Method, NetError, NetParseError,
    NetResult, RequestBuilder,
};
use crate::colors::{BLU, CLR, CYAN, GRN, PURP, RED, YLW};
use crate::config::TEST_SERVER_ADDR;
use crate::header_name::{CONNECTION, HOST};
use crate::util;

/// A shell-like TUI for an HTTP client.
#[derive(Debug)]
pub struct Tui<'out> {
    running: bool,
    do_send: bool,
    do_log_server: bool,
    client: Client,
    last_addr: Option<String>,
    last_code: Option<u16>,
    server: Option<Child>,
    out: BufWriter<StdoutLock<'out>>,
}

impl<'out> Tui<'out> {
    fn new() -> Self {
        Self {
            running: true,
            do_send: true,
            do_log_server: false,
            client: Client::default(),
            last_addr: None,
            last_code: None,
            server: None,
            out: BufWriter::new(io::stdout().lock())
        }
    }

    // Starts the TUI.
    pub fn run() {
        if let Err(e) = Self::run_main_loop() {
            eprintln!("{RED}Error: {e}{CLR}");
            process::exit(1);
        }

        process::exit(0);
    }

    // Runs the main IO loop.
    fn run_main_loop() -> NetResult<()> {
        let mut line = String::with_capacity(1024);

        let mut tui = Self::new();

        tui.print_intro()?;

        while tui.running {
            tui.print_prompt()?;

            line.clear();
            io::stdin().lock().read_line(&mut line)?;

            if let Err(e) = tui.handle_user_input(line.trim()) {
                writeln!(&mut tui.out, "{RED}{e}{CLR}")?;
            }
        }

        // Ensure test server is closed.
        tui.kill_server(true)?;

        // Print newline on exit.
        tui.out.write_all(b"\n")?;
        tui.out.flush()?;
        Ok(())
    }

    // Parses user input into a command or URI and handles execution of the
    // next steps.
    fn handle_user_input(&mut self, input: &str) -> NetResult<()> {
        match input {
            "" => {},
            "quit" => self.running = false,
            "clear" => self.clear_screen()?,
            "help" => self.print_help()?,
            "kill-server" => self.kill_server(false)?,
            "log-server" => self.toggle_server_logging()?,
            "start-server" => self.start_server()?,
            "body" | "request" | "response" | "minimal" | "verbose" => {
                self.output_style(input)?;
            },
            _ => {
                if self.parse_input(input).is_err() {
                    return Ok(());
                }

                if self.do_send {
                    self.send()?;
                    self.recv()?;
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
                }

                self.print_output()?;
            },
        }

        Ok(())
    }

    // Parses an input string into a request and adds it to the client.
    fn parse_input(&mut self, input: &str) -> NetResult<()> {
        let mut req = RequestBuilder::new();

        let uri = match input.split_once(' ') {
            None => input,
            Some((method, uri)) => {
                let method = method.to_ascii_uppercase();

                let Ok(method) = Method::from_str(&method) else {
                    self.warn_invalid_input(&method)?;
                    return Err(NetParseError::Method.into());
                };

                req.method(method);
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
            },
            None if uri.starts_with('/') && self.last_addr.is_some() => {
                let Some(addr) = self.last_addr.as_ref()
                else {
                    self.warn_invalid_input(uri)?;
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
            },
            None => {
                self.warn_invalid_input(uri)?;
                return Err(NetParseError::Path.into());
            },
        }

        self.client.req = Some(req.build());

        Ok(())
    }

    // Sets the output style and prints a message to stdout.
    fn output_style(&mut self, style: &str) -> NetResult<()> {
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

        let msg = format!("Output set to {PURP}{style}{CLR} style.\n");
        writeln!(&mut self.out, "{msg}")?;

        Ok(())
    }

    // Clears the screen and moves the cursor to the top left.
    fn clear_screen(&mut self) -> NetResult<()> {
        // Clear screen.
        self.out.write_all(b"\x1b[2J")?;

        // Move cursor to top left.
        self.out.write_all(b"\x1b[1;1H")?;
        Ok(())
    }

    // Prints the intro message on program start-up.
    fn print_intro(&mut self) -> NetResult<()> {
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


        self.clear_screen()?;

        writeln!(&mut self.out, "{PURP}http_tui/0.1\n\n{face}{CLR}\n")?;
        Ok(())
    }

    // Prints the prompt.
    fn print_prompt(&mut self) -> NetResult<()> {
        match self.last_code.take() {
            None => write!(&mut self.out, "{CYAN}${CLR} ")?,
            Some(code) => {
                let color = match code {
                    100..=199 | 300..=399 => BLU,
                    200..=299 => GRN,
                    400..=599 => RED,
                    _ => YLW,
                };

                let prompt = format!("{CYAN}[{color}{code}{CYAN}]${CLR} ");
                write!(&mut self.out, "{prompt}")?;
            },
        }

        self.out.flush()?;
        Ok(())
    }

    // Prints the help message to stdout.
    fn print_help(&mut self) -> NetResult<()> {
        writeln!(
            &mut self.out,
            "\
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
    clear         Clear the screen.
    help          Print this help message.
    kill-server   Shut down the test server.
    minimal       Only print the request lines and status lines.
    quit          Quit the program.
    request       Only print the requests (but do not send them).
    response      Only print the responses (default).
    start-server  Start a test server at localhost:7878.
    verbose       Print both requests and responses.\n"
        )?;
        Ok(())
    }

    // Print the request and response based on the output style settings.
    fn print_output(&mut self) -> NetResult<()> {
        self.client.print(&mut self.out)?;

        // Store status code for prompt.
        if let Some(res) = self.client.res.as_ref() {
            self.last_code = Some(res.status_code());
        }

        // Remove request and response after printing.
        self.client.req = None;
        self.client.res = None;

        Ok(())
    }

    fn warn_invalid_input(&mut self, input: &str) -> NetResult<()> {
        writeln!(&mut self.out, "{RED}Invalid input: \"{input}\"{CLR}\n")?;
        Ok(())
    }

    fn warn_no_connection(&mut self, addr: &str) -> NetResult<()> {
        // Reset prior connection.
        self.last_addr = None;
        self.client.req = None;
        self.client.res = None;
        self.client.conn = None;

        let msg = format!("{RED}Unable to connect to \"{addr}\"{CLR}\n");
        writeln!(&mut self.out, "{msg}")?;
        Ok(())
    }

    fn start_server(&mut self) -> NetResult<()> {
        if self.server.is_some() {
            let msg = format!(
                "{YLW}Server is already running.\nPlease run \
                `kill-server` to shut that server down first before \
                starting\na new one.{CLR}\n"
            );
            writeln!(&mut self.out, "{msg}")?;
            return Ok(());
        }

        if let Err(e) = util::build_server() {
            let msg = format!("{RED}Server failed to build: {e}{CLR}\n");
            writeln!(&mut self.out, "{msg}")?;
            return Ok(());
        }

        let args = [
            "run", "--bin", "server", "--", "--test", "--", TEST_SERVER_ADDR
        ];

        match Command::new("cargo")
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(server) => self.check_server_connection(server)?,
            Err(e) => {
                let msg = format!("{RED}Failed to start: {e}{CLR}\n");
                writeln!(&mut self.out, "{msg}")?;
            },
        }

        self.last_addr = Some(TEST_SERVER_ADDR.to_string());

        Ok(())
    }

    fn kill_server(&mut self, quiet: bool) -> NetResult<()> {
        let Some(mut server) = self.server.take() else {
            if !quiet {
                let msg = format!("{YLW}No active server found.{CLR}\n");
                writeln!(&mut self.out, "{msg}")?;
            }

            return Ok(());
        };

        Client::shutdown(TEST_SERVER_ADDR)?;

        match server.kill() {
            Ok(()) if !quiet => {
                let msg = format!("{GRN}Server has been shut down.{CLR}\n");
                writeln!(&mut self.out, "{msg}")?;
            },
            Err(e) if !quiet => {
                let msg = format!(
                    "{RED}Unable to shut down server.\n{e}{CLR}\n"
                );
                writeln!(&mut self.out, "{msg}")?;
            },
            _ => {},
        }

        self.last_addr = None;

        Ok(())
    }

    fn check_server_connection(&mut self, server: Child) -> NetResult<()> {
        let timeout = Duration::from_millis(200);

        let socket: SocketAddr = TEST_SERVER_ADDR
            .parse()
            .map_err(|_| NetError::NotConnected)?;

        // Attempt to connect a maximum of five times.
        for _ in 0..5 {
            if TcpStream::connect_timeout(&socket, timeout).is_ok() {
                self.server = Some(server);
                writeln!(&mut self.out,"{GRN}Server is running.{CLR}\n")?;
                return Ok(());
            }

            thread::sleep(timeout);
        }

        writeln!(&mut self.out, "{RED}Failed to start server.{CLR}\n")?;
        Ok(())
    }

    fn toggle_server_logging(&mut self) -> NetResult<()> {
        self.do_log_server = !self.do_log_server;

        let status = if self.do_log_server {
            "enabled"
        } else {
            "disabled"
        };

        let msg = format!("Server logging {PURP}{status}{CLR}.\n");
        writeln!(&mut self.out, "{msg}")?;

        Ok(())
    }

    fn connection_is_closed(&self) -> bool {
        self.client.res.as_ref().map_or(
            false,
            |res| {
                let value = res.headers.get(&CONNECTION);
                value == Some(&HeaderValue::from("close"))
            })
    }

    fn send(&mut self) -> NetResult<()> {
        if let Err(e) = self.client.send_request() {
            self.client.conn = None;
            let msg = format!(
                "{RED}Error while sending request:\n{}{CLR}\n",
                e.to_string().trim_end()
            );
            return Err(NetError::Other(msg.into()));
        }

        Ok(())
    }

    fn recv(&mut self) -> NetResult<()> {
        if let Err(e) = self.client.recv_response() {
            self.client.conn = None;

            let msg = format!(
                "{RED}Error while receiving response:\n{}{CLR}\n",
                e.to_string().trim_end()
            );

            return Err(NetError::Other(msg.into()));
        }

        if self.connection_is_closed() {
            self.client.conn = None;
        }

        Ok(())
    }
}

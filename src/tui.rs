use std::io::{self, BufRead, BufWriter, StdoutLock, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{self, Child, Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use crate::{
    Client, Connection, HeaderValue, Method, NetError, NetResult,
    RequestBuilder,
};
use crate::colors::{BLU, CLR, CYAN, GRN, PURP, RED, YLW};
use crate::header_name::CONNECTION;
use crate::util;

/// A shell-like TUI for an HTTP client.
#[derive(Debug)]
pub struct Tui<'out> {
    running: bool,
    do_send: bool,
    do_log_server: bool,
    client: Client,
    last_addr: String,
    server: Option<Child>,
    writer: BufWriter<StdoutLock<'out>>,
}

impl<'out> Tui<'out> {
    fn new() -> Self {
        Self {
            running: true,
            do_send: true,
            do_log_server: false,
            client: Client::default(),
            last_addr: String::new(),
            server: None,
            writer: BufWriter::new(io::stdout().lock())
        }
    }

    pub fn run() {
        if let Err(e) = Self::start_main_loop() {
            eprintln!("{RED}Error: {e}{CLR}");
            process::exit(1);
        }

        process::exit(0);
    }

    fn start_main_loop() -> NetResult<()> {
        let mut tui = Self::new();
        tui.print_intro()?;

        let mut line = String::with_capacity(1024);

        while tui.running {
            tui.print_prompt()?;

            line.clear();
            io::stdin().lock().read_line(&mut line)?;

            if let Err(e) = tui.handle_user_input(line.trim()) {
                writeln!(&mut tui.writer, "{RED}{e}{CLR}")?;
            }
        }

        // Ensure test server is closed.
        tui.kill_server(true)?;

        // Print newline on exit.
        tui.writer.write_all(b"\n")?;
        tui.writer.flush()?;
        Ok(())
    }

    fn handle_user_input(&mut self, input: &str) -> NetResult<()> {
        match input {
            "" => {},
            "quit" => self.running = false,
            "clear" => self.clear_screen()?,
            "help" => self.print_help()?,
            "kill-server" => self.kill_server(false)?,
            "log-server" => self.toggle_server_logging()?,
            "start-server" => self.start_server()?,
            "body" | "request" | "response" | "status" | "verbose" => {
                self.output_style(input)?;
            },
            uri => self.handle_uri(uri)?,
        }

        Ok(())
    }

    fn handle_uri(&mut self, input: &str) -> NetResult<()> {
        self.client.req = None;
        self.client.res = None;

        let mut req = RequestBuilder::new();

        let uri = match input.split_once(" ") {
            None => input,
            Some((method, uri)) => {
                let method = method.to_ascii_uppercase();

                let Ok(method) = Method::from_str(&method) else {
                    return self.warn_invalid_input(&method);
                };

                req.method(method);
                uri
            },
        };

        match util::parse_uri(uri).ok() {
            Some((addr, path)) => {
                // Only make a new connection if it is necessary.
                if self.client.conn.is_none() || self.last_addr != addr {
                    let Ok(conn) = Connection::try_from(addr.as_str()) else {
                        return self.warn_no_connection(&addr);
                    };

                    self.last_addr = addr;
                    self.client.conn = Some(conn);
                }

                req.path(&path);
            },
            None if uri.starts_with("/") => { req.path(uri); },
            None => return self.warn_invalid_input(uri),
        }

        self.client.req = Some(req.build());

        if self.do_send {
            self.send()?;
            self.recv()?;
        }

        self.print_output()?;
        Ok(())
    }

    // Clear the screen and move the cursor to the top left.
    fn clear_screen(&mut self) -> NetResult<()> {
        // Clear status code from prompt.
        self.client.res = None;

        // Clear screen.
        self.writer.write_all(b"\x1b[2J")?;

        // Move cursor to top left.
        self.writer.write_all(b"\x1b[1;1H")?;
        Ok(())
    }

    fn toggle_server_logging(&mut self) -> NetResult<()> {
        self.do_log_server = !self.do_log_server;

        let status = if self.do_log_server {
            "enabled"
        } else {
            "disabled"
        };

        writeln!(&mut self.writer, "Server logging {PURP}{status}{CLR}.\n")?;
        Ok(())
    }

    fn output_style(&mut self, style: &str) -> NetResult<()> {
        self.do_send = true;

        match style {
            "body" => self.client.output.format_str("b"),
            "response" => self.client.output.format_str("shb"),
            "status" => self.client.output.format_str("s"),
            "verbose" => self.client.output.format_str("*"),
            "request" => {
                self.do_send = false;
                self.client.output.format_str("r");
            },
            _ => unreachable!(),
        }

        writeln!(&mut self.writer,
            "Output set to {PURP}{style}{CLR} style.\n")?;

        Ok(())
    }

    fn print_intro(&mut self) -> NetResult<()> {
        const FACE: &str =
r#"              .-''''''-.
            .' _      _ '.
           /   O      O   \
          :                :
          |                |
          :       __       :
           \  .-"`  `"-.  /
            '.          .'
              '-......-'

         YOU SHOULDN'T BE HERE"#;


        self.clear_screen()?;
        writeln!(&mut self.writer, "{PURP}http_tui/0.1\n\n{FACE}{CLR}\n")?;
        Ok(())
    }

    fn warn_invalid_input(&mut self, input: &str) -> NetResult<()> {
        writeln!(&mut self.writer, "{RED}Invalid input: \"{input}\"{CLR}\n")?;
        Ok(())
    }

    fn warn_no_connection(&mut self, addr: &str) -> NetResult<()> {
        writeln!(&mut self.writer,
            "{RED}Unable to connect to \"{addr}\"{CLR}\n")?;
        Ok(())
    }

    fn print_prompt(&mut self) -> NetResult<()> {
        match self.client.res {
            None => write!(&mut self.writer, "{CYAN}${CLR} ")?,
            Some(ref res) => {
                let (color, code) = match res.status_code() {
                    code @ (100..=199 | 300..=399) => (BLU, code),
                    code @ 200..=299 => (GRN, code),
                    code @ 400..=599 => (RED, code),
                    code => (YLW, code),
                };

                write!(&mut self.writer,
                    "{CYAN}[{color}{code}{CYAN}]${CLR} ")?;
            },
        }

        self.writer.flush()?;
        Ok(())
    }

    fn print_help(&mut self) -> NetResult<()> {
        writeln!(&mut self.writer,
            "\
{PURP}http_tui{CLR} is a shell-like HTTP client.\n
Enter a `{PURP}URI{CLR}` to send a GET request.
    e.g. \"httpbin.org/status/201\"\n
To send a request with a different method enter `{PURP}METHOD URI{CLR}`.
    e.g. \"POST httpbin.org/status/201\"\n
Additional requests to the same address can be entered `{PURP}/PATH{CLR}`.
    e.g. \"/status/201\"\n
{PURP}COMMANDS:{CLR}
    body          Only print response bodies.
    clear         Clear the screen.
    help          Print this help message.
    kill-server   Shut down the test server.
    quit          Quit the program.
    request       Only print requests (but do not send them).
    response      Only print responses (default).
    start-server  Start a test server at localhost:7878.
    status        Only print status lines.
    verbose       Print both requests and responses.\n"
        )?;
        Ok(())
    }

    fn print_output(&mut self) -> NetResult<()> {
        self.client.print(&mut self.writer)?;
        Ok(())
    }

    fn start_server(&mut self) -> NetResult<()> {
        if self.server.is_some() {
            writeln!(
                &mut self.writer,
                "{YLW}Server is already running.\n\
                Please run `kill-server` to shut that server down first \
                before starting\na new one.{CLR}\n"
            )?;
            return Ok(());
        }

        if let Err(e) = util::build_server() {
            writeln!(
                &mut self.writer,
                "{RED}Server failed to build.\n{e}{CLR}\n"
            )?;
            return Ok(());
        }

        let args = [
            "run", "--bin", "server", "--", "--test", "--",
            "127.0.0.1:7878"
        ];

        match Command::new("cargo")
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(server) => self.check_server_connection(server)?,
            Err(e) => {
                writeln!(
                    &mut self.writer,
                    "{RED}Failed to start.\n{e}{CLR}\n"
                )?;
            },
        }

        Ok(())
    }

    fn kill_server(&mut self, quiet: bool) -> NetResult<()> {
        let Some(mut server) = self.server.take() else {
            if !quiet {
                writeln!(
                    &mut self.writer,
                    "{YLW}No active server found.{CLR}\n"
                )?;
            }
            return Ok(());
        };

        Client::shutdown("127.0.0.1:7878")?;

        match server.kill() {
            Ok(()) if !quiet => {
                writeln!(
                    &mut self.writer,
                    "{GRN}Server has been shut down.{CLR}\n"
                )?;
            },
            Err(e) if !quiet => {
                writeln!(
                    &mut self.writer,
                    "{RED}Unable to shut down server.\n{e}{CLR}\n"
                )?;
            },
            _ => {},
        }

        Ok(())
    }

    fn check_server_connection(&mut self, server: Child) -> NetResult<()> {
        let timeout = Duration::from_millis(200);

        let socket: SocketAddr = "127.0.0.1:7878"
            .parse()
            .map_err(|_| NetError::NotConnected)?;

        // Attempt to connect a maximum of five times.
        for _ in 0..5 {
            if TcpStream::connect_timeout(&socket, timeout).is_ok() {
                self.server = Some(server);
                writeln!(&mut self.writer,"{GRN}Server is running.{CLR}\n")?;
                return Ok(());
            }

            thread::sleep(timeout);
        }

        writeln!(&mut self.writer, "{RED}Failed to start server.{CLR}\n")?;
        Ok(())
    }

    fn connection_is_closed(&self) -> bool {
        match self.client.res.as_ref() {
            Some(res) => {
                res.headers.get(&CONNECTION) == Some(&HeaderValue::from("close"))
            },
            None => false,
        }
    }

    fn send(&mut self) -> NetResult<()> {
        if let Err(e) = self.client.send_request() {
            writeln!(
                &mut self.writer,
                "{RED}Error while sending request:\n{}{CLR}\n",
                e.to_string().trim_end()
            )?;
        }

        self.client.req = None;
        Ok(())
    }

    fn recv(&mut self) -> NetResult<()> {
        if let Err(e) = self.client.recv_response() {
            writeln!(
                &mut self.writer,
                "{RED}Error while receiving response:\n{}{CLR}\n",
                e.to_string().trim_end()
            )?;
        }

        if self.connection_is_closed() {
            self.client.conn = None;
        }

        Ok(())
    }
}

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{
    self, BufRead, BufWriter, Result as IoResult, StdinLock, StdoutLock,
    Write,
};

use rustnet::{Client, Connection, NetError, NetResult, Request, Response};
use rustnet::util;

// Ansi colors.
const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const YLW: &str = "\x1b[93m";
const CYAN: &str = "\x1b[96m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

// Output styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Style {
    Body,
    Request,
    Response,
    StatusLine,
    Verbose,
}

impl Display for Style {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Body => write!(f, "response body"),
            Self::Request => write!(f, "request"),
            Self::Response => write!(f, "response"),
            Self::StatusLine => write!(f, "status line"),
            Self::Verbose => write!(f, "verbose"),
        }
    }
}

// A simple shell-like TUI for an HTTP client.
#[derive(Debug)]
pub struct Tui<'a> {
    style: Style,
    running: bool,
    in_path_mode: bool,
    req: Option<Request>,
    res: Option<Response>,
    conn: Option<Connection>,
    stdin: StdinLock<'a>,
    stdout: BufWriter<StdoutLock<'a>>,
}

impl<'a> Tui<'a> {
    fn new(stdin: StdinLock<'a>, stdout: BufWriter<StdoutLock<'a>>) -> Self {
        Self {
            style: Style::Response,
            running: false,
            in_path_mode: false,
            req: None,
            res: None,
            conn: None,
            stdin,
            stdout,
        }
    }

    pub fn run() {
        let stdin = io::stdin().lock();
        let stdout = BufWriter::new(io::stdout().lock());
        let mut tui = Tui::new(stdin, stdout);

        if let Err(e) = tui.start_main_loop() {
            eprintln!("Error: {e}");
        }
    }

    fn start_main_loop(&mut self) -> NetResult<()> {
        let mut line = String::new();

        self.clr_screen()?;
        self.intro_msg()?;

        self.running = true;

        while self.running {
            self.home_prompt()?;

            line.clear();
            self.stdin.read_line(&mut line)?;

            match line.trim() {
                "" => continue,
                "close" | "quit" => self.running = false,
                "clear" => self.clr_screen()?,
                "help" => self.print_help()?,
                "home" => self.enter_home_mode()?,
                "body" => self.set_style(Style::Body)?,
                "request" => self.set_style(Style::Request)?,
                "response" => self.set_style(Style::Response)?,
                "status" => self.set_style(Style::StatusLine)?,
                "verbose" => self.set_style(Style::Verbose)?,
                uri if self.style == Style::Request => {
                    if let Ok((addr, path)) = util::parse_uri(uri) {
                        let mut client = Client::builder()
                            .addr(addr)
                            .path(&path)
                            .build()?;

                        self.req = client.req.take();
                    }

                    self.print_output()?;
                    self.req = None;
                },
                uri => match util::parse_uri(uri) {
                    Ok((addr, path)) => {
                        let mut client = Client::builder()
                            .addr(&addr)
                            .path(&path)
                            .build()?;

                        self.req = client.req.take();
                        self.conn = Some(client.conn);

                        self.send()?;
                        self.recv()?;

                        self.print_output()?;

                        if self.conn_is_closed() {
                            self.req = None;
                            self.res = None;
                            self.conn = None;
                        } else {
                            self.enter_path_mode(&addr)?;
                        }
                    },
                    Err(_) => self.invalid_input()?,
                },
            }
        }

        self.stdout.write_all(b"\n")?;
        self.stdout.flush()?;
        Ok(())
    }

    #[allow(unused)]
    fn check_for_command(&mut self, input: &str) -> NetResult<()> {
        match input {
            "close" | "quit" => self.running = false,
            "clear" => self.clr_screen()?,
            "help" => self.print_help()?,
            "home" => self.enter_home_mode()?,
            "body" => self.set_style(Style::Body)?,
            "request" => self.set_style(Style::Request)?,
            "response" => self.set_style(Style::Response)?,
            "status" => self.set_style(Style::StatusLine)?,
            "verbose" => self.set_style(Style::Verbose)?,
            _ => {},
        }
        Ok(())
    }

    fn enter_path_mode(&mut self, addr: &str) -> NetResult<()> {
        self.res = None;
        self.in_path_mode = true;

        let mut line = String::new();

        // This loop allows for us to keep using the same open connection.
        while self.in_path_mode {
            line.clear();
            self.path_prompt(addr)?;
            self.stdin.read_line(&mut line)?;

            match line.trim() {
                "" => continue,
                path if path.starts_with('/') => {
                    self.set_path(path);

                    if self.style == Style::Request {
                        self.print_output()?;
                    } else {
                        self.send()?;
                        self.recv()?;

                        self.print_output()?;

                        self.in_path_mode = !self.conn_is_closed();
                        self.res = None;
                    }
                },
                "close" | "quit" => {
                    self.in_path_mode = false;
                    self.running = false;
                },
                "clear" => self.clr_screen()?,
                "help" => self.print_help()?,
                "home" => self.enter_home_mode()?,
                "body" => self.set_style(Style::Body)?,
                "request" => self.set_style(Style::Request)?,
                "response" => self.set_style(Style::Response)?,
                "status" => self.set_style(Style::StatusLine)?,
                "verbose" => self.set_style(Style::Verbose)?,
                _ => self.invalid_input()?,
            }
        }
        Ok(())
    }

    // Clear the screen and move the cursor to the top left.
    fn clr_screen(&mut self) -> NetResult<()> {
        self.stdout.write_all(b"\x1b[2J\x1b[1;1H")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn intro_msg(&mut self) -> NetResult<()> {
        writeln!(
            &mut self.stdout,
            "{CYAN}tui_client{CLR} is an HTTP client.\n\
			Enter `{YLW}help{CLR}` to see all options.\n"
        )?;
        self.stdout.flush()?;
        Ok(())
    }

    fn enter_home_mode(&mut self) -> NetResult<()> {
        self.in_path_mode = false;
        self.res = None;
        Ok(())
    }

    fn set_style(&mut self, style: Style) -> NetResult<()> {
        self.style = style;
        writeln!(&mut self.stdout, "Output set to: {CYAN}{}{CLR}\n", &self.style)?;
        self.stdout.flush()?;
        Ok(())
    }

    fn home_prompt(&mut self) -> NetResult<()> {
        write!(&mut self.stdout, "{GRN}[HOME]${CLR} ")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn path_prompt(&mut self, addr: &str) -> NetResult<()> {
        write!(&mut self.stdout, "{YLW}[{addr}]${CLR} ")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn print_help(&mut self) -> NetResult<()> {
        writeln!(
            &mut self.stdout,
            "\
{PURP}HELP:{CLR}
    Enter an HTTP URI ({GRN}HOME{CLR} mode) or a URI path ({YLW}PATH{CLR} mode) to send
    an HTTP request to a remote host.\n
{PURP}MODES:{CLR}
    {GRN}Home{CLR}      Enter an HTTP URI to send a request.
              Example:
              {GRN}[HOME]${CLR} httpbin.org/encoding/utf8\n
    {YLW}Path{CLR}      Enter a URI path to send a new request to the same host.
              This mode is entered automatically while the connection
              to the remote host is kept alive. It can be manually
              exited by using the `home` command.
              Example:
              {YLW}[httpbin.org:80]${CLR} /encoding/utf8\n
{PURP}COMMANDS:{CLR}
    body      Print data from response bodies.
    clear     Clear the terminal.
    close     Close the program.
    help      Print this help message.
    home      Exit {YLW}PATH{CLR} mode and return to {GRN}HOME{CLR} mode.
    request   Print requests (but do not send them).
    response  Print responses (default).
    status    Print response status lines.
    verbose   Print both the requests and the responses.\n")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn set_path(&mut self, path: &str) {
        if let Some(req) = self.req.as_mut() {
            req.request_line.path = path.to_string();
        }
    }

    fn invalid_input(&mut self) -> NetResult<()> {
        writeln!(&mut self.stdout, "{RED}Invalid input.{CLR}")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn send(&mut self) -> NetResult<()> {
        let mut writer = self.conn
            .as_ref()
            .ok_or(NetError::NotConnected)
            .and_then(|conn| conn.writer.try_clone())?;

        self.req
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(|req| writer.send_request(req))?;

        Ok(())
    }

    fn recv(&mut self) -> NetResult<()> {
        let res = self.conn
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(|conn| conn.reader.recv_response())?;

        self.res = Some(res);
        Ok(())
    }

    fn get_status_line(&mut self, output: &mut String) {
        if let Some(res) = self.res.as_ref() {
            output.push_str(&res.status_line.to_color_string());
        } else {
            output.push_str("No response found.\n");
        }
    }

    fn get_res_body(&mut self, output: &mut String) {
        if let Some(res) = self.res.as_ref() {
            if res.body.is_printable() {
                let body = String::from_utf8_lossy(res.body.as_bytes());
                output.push('\n');
                output.push_str(body.trim_end());
            } else if !res.body.is_empty() {
                output.push_str("Body cannot be printed.\n");
            }
        } else {
            output.push_str("No response found.\n");
        }
    }

    fn get_request(&mut self, output: &mut String) {
        if let Some(req) = self.req.as_ref() {
            output.push('\n');
            output.push_str(&req.request_line.to_color_string());
            output.push_str(&req.headers.to_color_string());
            
            if req.body.is_printable() {
                output.push('\n');
                let body = String::from_utf8_lossy(req.body.as_bytes());
                output.push_str(body.trim_end());
            } else if !req.body.is_empty() {
                output.push_str("Body cannot be printed.\n");
            }
        } else {
            output.push_str("No request found.\n");
        }
    }

    fn get_response(&mut self, output: &mut String) {
        if let Some(res) = self.res.as_ref() {
            output.push('\n');
            output.push_str(&res.status_line.to_color_string());
            output.push_str(&res.headers.to_color_string());

            if res.body.is_printable() {
                output.push('\n');
                let body = String::from_utf8_lossy(res.body.as_bytes());
                output.push_str(body.trim_end());
            } else if !res.body.is_empty() {
                output.push_str("Body cannot be printed.\n");
            }
        } else {
            output.push_str("No response found.\n");
        }
    }

    fn get_verbose(&mut self, output: &mut String) {
        self.get_request(output);
        self.get_response(output);
    }

    fn print_output(&mut self) -> IoResult<()> {
        let mut output = String::new();

        match self.style {
            Style::StatusLine => self.get_status_line(&mut output),
            Style::Body => self.get_res_body(&mut output),
            Style::Request => self.get_request(&mut output),
            Style::Response => self.get_response(&mut output),
            Style::Verbose => self.get_verbose(&mut output),
        }

        self.stdout.write_all(output.as_bytes())?;
        self.stdout.write_all(b"\n\n")?;
        self.stdout.flush()
    }

    fn conn_is_closed(&self) -> bool {
        self.res
            .as_ref()
            .map_or(true, |res| res.has_close_connection_header())
    }
}

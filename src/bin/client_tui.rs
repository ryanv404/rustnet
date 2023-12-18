use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{self, BufRead, BufWriter, StdinLock, StdoutLock, Write};

use rustnet::{Client, NetError, NetReader, NetResult, NetWriter, Request, Response};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const YLW: &str = "\x1b[93m";
const CYAN: &str = "\x1b[96m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

#[derive(Debug, PartialEq, Eq)]
// Output style options.
enum OutputStyle {
    Status,
    Request,
    ResBody,
    Response,
    Verbose,
}

impl Display for OutputStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Status => write!(f, "status"),
            Self::ResBody => write!(f, "body"),
            Self::Request => write!(f, "request"),
            Self::Response => write!(f, "response"),
            Self::Verbose => write!(f, "verbose"),
        }
    }
}

// An HTTP TUI client.
#[derive(Debug)]
pub struct Browser<'a> {
    is_running: bool,
    in_path_mode: bool,
    output_style: OutputStyle,
    request: Option<Request>,
    response: Option<Response>,
    reader: Option<NetReader>,
    writer: Option<NetWriter>,
    stdin: StdinLock<'a>,
    stdout: BufWriter<StdoutLock<'a>>,
}

impl<'a> Browser<'a> {
    fn new(stdin: StdinLock<'a>, stdout: BufWriter<StdoutLock<'a>>) -> Self {
        Self {
            is_running: false,
            in_path_mode: false,
            output_style: OutputStyle::Response,
            request: None,
            response: None,
            reader: None,
            writer: None,
            stdin,
            stdout,
        }
    }

    pub fn run_client_tui() -> NetResult<()> {
        let stdin = io::stdin().lock();
        let stdout = BufWriter::new(io::stdout().lock());
        let mut browser = Browser::new(stdin, stdout);

        browser.clear_screen()?;
        browser.print_intro_message()?;

        if let Err(e) = browser.run() {
            eprintln!("Error: {e}");
        }

        browser.stdout.write_all(b"\n")?;
        browser.stdout.flush()?;
        Ok(())
    }

    fn run(&mut self) -> NetResult<()> {
        let mut line = String::new();

        self.is_running = true;

        while self.is_running {
            line.clear();

            self.print_home_prompt()?;
            self.stdin.read_line(&mut line)?;

            match line.trim() {
                "" => continue,
                "body" => {
                    self.set_output_style(OutputStyle::ResBody)?;
                }
                "clear" => {
                    self.clear_screen()?;
                }
                "close" | "quit" => self.is_running = false,
                "help" => {
                    self.show_help()?;
                }
                "home" => {
                    self.set_home_mode()?;
                }
                "request" => {
                    self.set_output_style(OutputStyle::Request)?;
                }
                "response" => {
                    self.set_output_style(OutputStyle::Response)?;
                }
                "status" => {
                    self.set_output_style(OutputStyle::Status)?;
                }
                "verbose" => {
                    self.set_output_style(OutputStyle::Verbose)?;
                }
                uri if self.output_style == OutputStyle::Request => {
                    if let Ok((addr, path)) = Client::parse_uri(uri) {
                        let mut client = Client::builder()
                            .addr(addr).path(&path).build()?;

                        self.request = client.req.take();
                    }

                    self.print_request()?;
                    self.request = None;
                }
                uri => match Client::parse_uri(uri) {
                    Ok((addr, path)) => {
                        let mut client = Client::builder()
                            .addr(&addr).path(&path).build()?;

                        self.reader = client.reader.try_clone().ok();
                        self.writer = client.writer.try_clone().ok();
                        self.request = client.req.take();

                        self.send()?;
                        self.recv()?;
                        self.print_output()?;

                        if self.is_connection_open() {
                            self.run_path_mode(&addr)?;
                        } else {
                            self.request = None;
                            self.response = None;
                            self.reader = None;
                            self.writer = None;
                        }
                    }
                    Err(_) => {
                        self.warn_invalid_input("URI")?;
                    }
                },
            }
        }

        Ok(())
    }

    #[allow(unused)]
    fn check_for_command(&mut self, input: &str) -> NetResult<()> {
        match input {
            "body" => {
                self.set_output_style(OutputStyle::ResBody)?;
            }
            "clear" => {
                self.clear_screen()?;
            }
            "close" | "quit" => self.is_running = false,
            "help" => {
                self.show_help()?;
            }
            "home" => {
                self.set_home_mode()?;
            }
            "request" => {
                self.set_output_style(OutputStyle::Request)?;
            }
            "response" => {
                self.set_output_style(OutputStyle::Response)?;
            }
            "status" => {
                self.set_output_style(OutputStyle::Status)?;
            }
            "verbose" => {
                self.set_output_style(OutputStyle::Verbose)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn run_path_mode(&mut self, addr: &str) -> NetResult<()> {
        let mut line = String::new();

        self.response = None;
        self.in_path_mode = true;

        // This loop allows for us to keep using the same open connection.
        while self.in_path_mode {
            line.clear();

            self.print_path_prompt(addr)?;
            self.stdin.read_line(&mut line)?;

            match line.trim() {
                "" => continue,
                path if path.starts_with('/') => {
                    self.set_path(path);

                    if self.output_style == OutputStyle::Request {
                        self.print_request()?;
                    } else {
                        self.send()?;
                        self.recv()?;
                        self.print_output()?;
                        self.in_path_mode = self.is_connection_open();
                        self.response = None;
                    }
                }
                "body" => {
                    self.set_output_style(OutputStyle::ResBody)?;
                }
                "clear" => {
                    self.clear_screen()?;
                }
                "close" | "quit" => {
                    self.in_path_mode = false;
                    self.is_running = false;
                }
                "help" => {
                    self.show_help()?;
                }
                "home" => {
                    self.set_home_mode()?;
                }
                "request" => {
                    self.set_output_style(OutputStyle::Request)?;
                }
                "response" => {
                    self.set_output_style(OutputStyle::Response)?;
                }
                "status" => {
                    self.set_output_style(OutputStyle::Status)?;
                }
                "verbose" => {
                    self.set_output_style(OutputStyle::Verbose)?;
                }
                _ => {
                    self.warn_invalid_input("path")?;
                }
            }
        }

        Ok(())
    }

    fn clear_screen(&mut self) -> NetResult<()> {
        // Clear the screen and move the cursor to the top left.
        self.stdout.write_all(b"\x1b[2J\x1b[1;1H")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn print_intro_message(&mut self) -> NetResult<()> {
        let prog_name = env!("CARGO_BIN_NAME");
        writeln!(
            &mut self.stdout,
            "`{CYAN}{prog_name}{CLR}` is an HTTP client.\n\
			Enter `{YLW}help{CLR}` to see all options.\n"
        )?;
        self.stdout.flush()?;
        Ok(())
    }

    fn set_home_mode(&mut self) -> NetResult<()> {
        self.in_path_mode = false;
        self.response = None;
        Ok(())
    }

    fn set_output_style(&mut self, style: OutputStyle) -> NetResult<()> {
        self.output_style = style;
        writeln!(
            &mut self.stdout,
            "Output style: {CYAN}{}{CLR}\n",
            self.output_style
        )?;
        self.stdout.flush()?;
        Ok(())
    }

    fn print_home_prompt(&mut self) -> NetResult<()> {
        write!(&mut self.stdout, "{GRN}[HOME]${CLR} ")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn print_path_prompt(&mut self, addr: &str) -> NetResult<()> {
        write!(&mut self.stdout, "{YLW}[{addr}]${CLR} ")?;
        self.stdout.flush()?;
        Ok(())
    }

    fn show_help(&mut self) -> NetResult<()> {
        writeln!(
            &mut self.stdout,
            "\n\
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
    verbose   Print both the requests and the responses.\n"
        )?;

        Ok(())
    }

    fn set_path(&mut self, path: &str) {
        if let Some(req) = self.request.as_mut() {
            req.request_line.path = path.to_string();
        }
    }

    fn warn_invalid_input(&mut self, kind: &str) -> NetResult<()> {
        writeln!(
            &mut self.stdout,
            "{RED}Not a valid {kind} or command.{CLR}\n"
        )?;
        self.stdout.flush()?;
        Ok(())
    }

    fn send(&mut self) -> NetResult<()> {
        let mut writer = self
            .writer
            .as_ref()
            .ok_or(NetError::NotConnected)
            .and_then(|writer| writer.try_clone())?;

        self.request
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(|req| writer.send_request(req))?;

        Ok(())
    }

    fn recv(&mut self) -> NetResult<()> {
        self.response = self
            .reader
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(|reader| reader.recv_response())
            .ok();

        Ok(())
    }

    fn print_request(&mut self) -> NetResult<()> {
        let output = self.request.as_ref().map_or_else(
            || String::from("No request.\n"),
            |req| {
                let req_line = req.request_line();
                let headers = req.headers_to_string();
                let headers = headers.trim_end();

                if req.body.is_empty() {
                    format!("\n{PURP}{req_line}{CLR}\n{headers}\n\n")
                } else {
                    let body = String::from_utf8_lossy(req.body.as_bytes());
                    format!(
                        "\n{PURP}{req_line}{CLR}\n{headers}\n\n{}\n\n",
                        body.trim_end()
                    )
                }
            },
        );

        self.stdout.write_all(output.as_bytes())?;
        self.stdout.flush()?;
        Ok(())
    }

    fn print_response_body(&mut self) -> NetResult<()> {
        let output = self.response.as_ref().map_or_else(
            || String::from("No response.\n"),
            |res| {
                if res.body.is_empty() {
                    String::from("No response body data.\n")
                } else {
                    let body = String::from_utf8_lossy(res.body.as_bytes());
                    format!("\n{}\n\n", body.trim_end())
                }
            },
        );

        self.stdout.write_all(output.as_bytes())?;
        self.stdout.flush()?;
        Ok(())
    }

    fn print_response(&mut self) -> NetResult<()> {
        let output = self.response.as_ref().map_or_else(
            || String::from("No response.\n"),
            |res| {
                let status_line = res.status_line();
                let headers = res.headers_to_string();
                let headers = headers.trim_end();
                let color = if res.status_code() >= 400 { RED } else { GRN };

                if res.body.is_empty() {
                    format!("\n{color}{status_line}{CLR}\n{headers}\n\n")
                } else {
                    let body = String::from_utf8_lossy(res.body.as_bytes());
                    format!(
                        "\n{color}{status_line}{CLR}\n{headers}\n\n{}\n\n",
                        body.trim_end()
                    )
                }
            },
        );

        self.stdout.write_all(output.as_bytes())?;
        self.stdout.flush()?;
        Ok(())
    }

    fn print_status_line(&mut self) -> NetResult<()> {
        let output = self.response.as_ref().map_or_else(
            || String::from("No status line.\n"),
            |res| {
                let color = if res.status_code() >= 400 { RED } else { GRN };
                format!("{color}{}{CLR}\n\n", res.status_line())
            },
        );

        self.stdout.write_all(output.as_bytes())?;
        self.stdout.flush()?;
        Ok(())
    }

    fn print_output(&mut self) -> NetResult<()> {
        match self.output_style {
            OutputStyle::Status => {
                self.print_status_line()?;
            }
            OutputStyle::Request => {
                self.print_request()?;
            }
            OutputStyle::ResBody => {
                self.print_response_body()?;
            }
            OutputStyle::Response => {
                self.print_response()?;
            }
            OutputStyle::Verbose => {
                self.print_request()?;
                self.print_response()?;
            }
        }

        Ok(())
    }

    fn is_connection_open(&self) -> bool {
        !self.response
            .as_ref()
            .map_or(false, |res| res.has_closed_connection_header())
    }
}

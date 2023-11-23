use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{self, BufRead, BufWriter, StdinLock, StdoutLock, Write};

use librustnet::{Client, Response};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const YLW: &str = "\x1b[93m";
const CYAN: &str = "\x1b[96m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

fn main() {
    let stdin = io::stdin().lock();
    let stdout = BufWriter::new(io::stdout().lock());
    let mut browser = Browser::new(stdin, stdout);

    browser.clear_screen();
    browser.print_intro_message();
    browser.run();
}

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

// An HTTP client.
#[derive(Debug)]
struct Browser<'a> {
    is_running: bool,
    in_path_mode: bool,
    output_style: OutputStyle,
    client: Option<Client>,
    response: Option<Response>,
    stdin: StdinLock<'a>,
    stdout: BufWriter<StdoutLock<'a>>,
}


impl<'a> Browser<'a> {
    fn new(stdin: StdinLock<'a>, stdout: BufWriter<StdoutLock<'a>>) -> Self {
        Self {
            is_running: false,
            in_path_mode: false,
            output_style: OutputStyle::Response,
            client: None,
            response: None,
            stdin,
            stdout,
        }
    }

    fn run(&mut self) {
        self.is_running = true;
        let mut line = String::new();

        while self.is_running {
            line.clear();
            self.print_home_prompt();
            self.stdin.read_line(&mut line).unwrap();

            match line.trim() {
                "" => continue,
                "body" => self.set_output_style(OutputStyle::ResBody),
                "clear" => self.clear_screen(),
                "close" | "quit" => self.is_running = false,
                "help" => self.show_help(),
                "home" => self.set_home_mode(),
                "request" => self.set_output_style(OutputStyle::Request),
                "response" => self.set_output_style(OutputStyle::Response),
                "status" => self.set_output_style(OutputStyle::Status),
                "verbose" => self.set_output_style(OutputStyle::Verbose),
                uri if self.output_style == OutputStyle::Request => {
                    self.client = Client::parse_uri(uri).and_then(
                        |(addr, path)| {
                            Client::http().addr(addr).path(&path).build().ok()
                        }
                    );
                    self.print_request();
                    self.client = None;
                },
                uri => match Client::get(uri) {
                    Ok((client, res, addr)) => {
                        self.client = Some(client);
                        self.response = Some(res);
                        self.print_output();

                        if self.is_connection_open() {
                            self.run_path_mode(&addr);
                        } else {
                            self.response = None;
                        }
                    },
                    Err(_) => self.warn_invalid_input("URI"),
                }
            }
        }
    }

    #[allow(unused)]
    fn check_for_command(&mut self, input: &str) {
        match input {
            "body" => self.set_output_style(OutputStyle::ResBody),
            "clear" => self.clear_screen(),
            "close" | "quit" => self.is_running = false,
            "help" => self.show_help(),
            "home" => self.set_home_mode(),
            "request" => self.set_output_style(OutputStyle::Request),
            "response" => self.set_output_style(OutputStyle::Response),
            "status" => self.set_output_style(OutputStyle::Status),
            "verbose" => self.set_output_style(OutputStyle::Verbose),
            _ => {},
        }
    }

    fn run_path_mode(&mut self, addr: &str) {
        self.response = None;
        self.in_path_mode = true;
        let mut line = String::new();

        // This loop allows for us to keep using the same open connection.
        while self.in_path_mode {
            line.clear();
            self.print_path_prompt(addr);
            self.stdin.read_line(&mut line).unwrap();

            match line.trim() {
                "" => continue,
                path if path.starts_with('/') => {
                    self.set_path(path);

                    if self.output_style == OutputStyle::Request {
                        self.print_request();
                    } else {
                        self.send();
                        self.recv();

                        if self.response.is_some() {
                            self.print_output();
                            self.in_path_mode = self.is_connection_open();
                            self.response = None;
                        }
                    }
                },
                "body" => self.set_output_style(OutputStyle::ResBody),
                "clear" => self.clear_screen(),
                "close" | "quit" => {
                    self.in_path_mode = false;
                    self.is_running = false;
                },
                "help" => self.show_help(),
                "home" => self.set_home_mode(),
                "request" => self.set_output_style(OutputStyle::Request),
                "response" => self.set_output_style(OutputStyle::Response),
                "status" => self.set_output_style(OutputStyle::Status),
                "verbose" => self.set_output_style(OutputStyle::Verbose),
                _ => self.warn_invalid_input("path"),
            }
        }
    }

    fn clear_screen(&mut self) {
		// Clear the screen and move the cursor to the top left.
        self.stdout.write_all(b"\x1b[2J\x1b[1;1H").unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_intro_message(&mut self) {
		writeln!(
		    &mut self.stdout,
		    "{CYAN}{}{CLR} is an HTTP client.\n\
			Enter `{YLW}help{CLR}` to see all options.\n", 
			env!("CARGO_BIN_NAME")
		).unwrap();
        self.stdout.flush().unwrap();
    }

    fn set_home_mode(&mut self) {
        self.in_path_mode = false;
        self.client = None;
        self.response = None;
    }

    fn set_output_style(&mut self, style: OutputStyle) {
        self.output_style = style;
        writeln!(
            &mut self.stdout,
            "Output style: `{CYAN}{}{CLR}`.\n",
            self.output_style
        ).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_home_prompt(&mut self) {
        write!(&mut self.stdout, "{GRN}[HOME]${CLR} ").unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_path_prompt(&mut self, addr: &str) {
        write!(&mut self.stdout, "{YLW}[{addr}]${CLR} ").unwrap();
        self.stdout.flush().unwrap();
    }

    fn show_help(&mut self) {
        writeln!(&mut self.stdout, "\n\
{PURP}Help:{CLR}
    Enter an HTTP URI ({GRN}HOME{CLR} mode) or a URI path ({YLW}PATH{CLR} mode) to send
    an HTTP request to a remote host.\n
{PURP}Modes:{CLR}
    {GRN}Home{CLR}      Enter an HTTP URI to send a request.
              Example: {GRN}[HOME]${CLR} httpbin.org/encoding/utf8\n
    {YLW}Path{CLR}      Enter a URI path to send a new request to the same host.
              This mode is entered automatically while the connection
              to the remote host is kept alive. It can be manually
              exited by using the `home` command.
              Example: {YLW}[httpbin.org:80]${CLR} /encoding/utf8\n
{PURP}Commands:{CLR}
    body      Print data from response bodies.
    clear     Clear the terminal.
    close     Close the program.
    help      Print this help message.
    home      Exit {YLW}PATH{CLR} mode and return to {GRN}HOME{CLR} mode.
    request   Print requests (but do not send them).
    response  Print responses (default).
    status    Print response status lines.
    verbose   Print both the requests and the responses.\n").unwrap();
    }

    fn set_path(&mut self, path: &str) {
        if let Some(client) = self.client.as_mut() {
            client.req.path = path.to_string();
        }
    }

    fn warn_invalid_input(&mut self, kind: &str) {
        writeln!(
            &mut self.stdout,
            "{RED}Not a valid {kind} or command.{CLR}\n"
        ).unwrap();
        self.stdout.flush().unwrap();
    }

    fn send(&mut self) {
        if let Some(client) = self.client.as_mut() {
            client.send().unwrap();
        }
    }

    fn recv(&mut self) {
        if let Some(client) = self.client.as_mut() {
            self.response = client.recv().ok();
        }
    }

    fn print_request(&mut self) {
        let output = self.client.as_ref().map_or_else(
            || String::from("No request.\n"),
            |client| {
                let req_line = client.req.request_line();
                let headers = client.req.headers_to_string();
                let headers = headers.trim_end();

                client.req.body.as_ref().map_or_else(
                    || format!("\n{PURP}{req_line}{CLR}\n{headers}\n\n"),
                    |body| {
                        let body = String::from_utf8_lossy(body);
                        format!(
                            "\n{PURP}{req_line}{CLR}\n{headers}\n\n{}\n\n",
                            body.trim_end()
                        )
                    }
                )
            }
        );
        self.stdout.write_all(output.as_bytes()).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_response_body(&mut self) {
        let output = self.response.as_ref().map_or_else(
            || String::from("No response.\n"),
            |res| res.body.as_ref().map_or_else(
                || String::from("No response body data.\n"),
                |body| {
                    let body = String::from_utf8_lossy(body);
                    format!("\n{}\n\n", body.trim_end())
                })
        );
        self.stdout.write_all(output.as_bytes()).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_response(&mut self) {
        let output = self.response.as_ref().map_or_else(
            || String::from("No response.\n"),
            |res| {
                let status_line = res.status_line();
                let headers = res.headers_to_string();
                let headers = headers.trim_end();
                let color = if res.status_code() >= 400 { RED } else { GRN };
                res.body.as_ref().map_or_else(
                    || format!("\n{color}{status_line}{CLR}\n{headers}\n\n"),
                    |body| {
                        let body = String::from_utf8_lossy(body);
                        format!(
                            "\n{color}{status_line}{CLR}\n{headers}\n\n{}\n\n",
                            body.trim_end()
                        )
                    })
            }
        );
        self.stdout.write_all(output.as_bytes()).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_status_line(&mut self) {
        let output = self.response.as_ref().map_or_else(
            || String::from("No status line.\n"),
            |res| {
                let color = if res.status_code() >= 400 { RED } else { GRN };
                format!("{color}{}{CLR}\n\n", res.status_line())
            }
        );
        self.stdout.write_all(output.as_bytes()).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_output(&mut self) {
        match self.output_style {
            OutputStyle::Status => {
                self.print_status_line();
            },
            OutputStyle::Request => {
                self.print_request();
            },
            OutputStyle::ResBody => {
                self.print_response_body();
            },
            OutputStyle::Response => {
                self.print_response();
            },
            OutputStyle::Verbose => {
                self.print_request();
                self.print_response();
            },
        }
    }

    fn is_connection_open(&self) -> bool {
        self.response.as_ref().map_or(false, |res| {
            !res.has_close_connection_header()
        })
    }
}

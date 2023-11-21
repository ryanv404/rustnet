use std::fmt;
use std::io::{self, BufRead, StdinLock, StdoutLock, Write};

use rustnet::{Client, Response};

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const YLW: &str = "\x1b[93m";
const CYAN: &str = "\x1b[96m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

fn main() {
    let mut line = String::new();
    let stdin = io::stdin().lock();
    let stdout = io::stdout().lock();

    let mut browser = Browser::new(stdin, stdout);
    browser.clear_screen();
    browser.print_intro_message();

    'outer: loop {
        line.clear();
        browser.print_home_prompt();
        browser.stdin.read_line(&mut line).unwrap();

        match line.trim() {
            "" => continue,
            "body" => browser.set_output_style(OutputStyle::ResBody),
            "close" | "quit" => break 'outer,
            "clear" => browser.clear_screen(),
            "help" => browser.show_help(),
            "home" => browser.set_home_mode(),
            "request" => browser.set_output_style(OutputStyle::Request),
            "response" => browser.set_output_style(OutputStyle::Response),
            "status" => browser.set_output_style(OutputStyle::Status),
            "verbose" => browser.set_output_style(OutputStyle::Verbose),
            uri if browser.output_style == OutputStyle::Request => {
                if let Some((addr, path)) = Client::parse_uri(uri) {
                    browser.client = Client::http()
                        .addr(addr)
                        .path(&path)
                        .build()
                        .ok();
                }

                browser.print_request();
                browser.client = None;
            }
            uri => match Client::get(uri) {
                Ok((client, res, addr)) => {
                    browser.client = Some(client);
                    browser.response = Some(res);

                    if browser.response.is_some() {
                        browser.print_output();
                        browser.in_path_mode = !browser.connection_closed();
                        browser.response = None;
                    }

                    // This loop allows for us to keep using the same open connection.
                    while browser.in_path_mode {
                        line.clear();
                        browser.print_path_prompt(&addr);
                        browser.stdin.read_line(&mut line).unwrap();

                        match line.trim() {
                            "" => continue,
                            "body" => browser.set_output_style(OutputStyle::ResBody),
                            "close" | "quit" => break 'outer,
                            "clear" => browser.clear_screen(),
                            "help" => browser.show_help(),
                            "home" => browser.set_home_mode(),
                            "request" => browser.set_output_style(OutputStyle::Request),
                            "response" => browser.set_output_style(OutputStyle::Response),
                            "status" => browser.set_output_style(OutputStyle::Status),
                            "verbose" => browser.set_output_style(OutputStyle::Verbose),
                            path if path.starts_with('/') => {
                                browser.set_path(path);
                                if browser.output_style == OutputStyle::Request {
                                    browser.print_request();
                                } else {
                                    browser.send();
                                    browser.recv();

                                    if browser.response.is_some() {
                                        browser.print_output();
                                        browser.in_path_mode = !browser.connection_closed();
                                        browser.response = None;
                                    }
                                }
                            },
                            _ => browser.warn_invalid_input(),
                        }
                    }
                },
                Err(_) => browser.warn_invalid_input(),
            }
        }
    }
}

const HELP_MSG: &str = "
\x1b[95mHELP:\x1b[0m
    Enter an HTTP URI (\x1b[92mhome\x1b[0m mode) or a URI path (\x1b[93mpath\x1b[0m mode) to send
    an HTTP request to a remote host.\n
\x1b[95mMODES:\x1b[0m
    \x1b[92mHome\x1b[0m      Enter an HTTP URI to send a request.
              Example:
              \x1b[92m[HOME]$\x1b[0m httpbin.org/encoding/utf8\n
    \x1b[93mPath\x1b[0m      Enter a URI path to send a new request to the same host.
              This mode is entered automatically while the connection
              to the remote host is kept alive. It can be manually
              exited by using the `home` command.
              Example:
              \x1b[93m[httpbin.org:80]$\x1b[0m /encoding/utf8\n
\x1b[95mCOMMANDS:\x1b[0m
    body      Only print the response body (default).
    clear     Clear the terminal.
    close     Close the program.
    help      Print this help message.
    home      Exit \x1b[93mpath\x1b[0m mode and return to \x1b[92mhome\x1b[0m mode.
    request   Only print the request (does not send the request).
    response  Only print the response.
    status    Only print the response status line.
    verbose   Print both the request and the response.
";

#[derive(Debug, PartialEq, Eq)]
// Output style options.
enum OutputStyle {
    Status,
    Request,
    ResBody,
    Response,
    Verbose,
}

impl fmt::Display for OutputStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    output_style: OutputStyle,
    in_path_mode: bool,
    client: Option<Client>,
    response: Option<Response>,
    stdin: StdinLock<'a>,
    stdout: StdoutLock<'a>,
}


impl<'a> Browser<'a> {
    fn new(stdin: StdinLock<'a>, stdout: StdoutLock<'a>) -> Self {
        Self {
            output_style: OutputStyle::ResBody,
            in_path_mode: false,
            client: None,
            response: None,
            stdin,
            stdout,
        }
    }

    fn clear_screen(&mut self) {
		// Clear the screen and move the cursor to the top left.
        self.stdout.write_all(b"\x1b[2J\x1b[1;1H").unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_intro_message(&mut self) {
		let name = env!("CARGO_BIN_NAME");
		let msg = format!("\
			{CYAN}{name}{CLR} is an HTTP client.\n\
			Enter `{YLW}help{CLR}` to see all options.\n\
		");

        writeln!(&mut self.stdout, "{msg}").unwrap();
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
            "Output style set to `{CYAN}{}{CLR}`.\n",
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
        writeln!(&mut self.stdout, "{HELP_MSG}").unwrap();
    }

    fn set_path(&mut self, path: &str) {
        self.client.as_mut().map(|client| {
            client.req.path = path.to_string();
        });
    }

    fn warn_invalid_input(&mut self) {
        writeln!(&mut self.stdout, "{RED}Invalid input.{CLR}").unwrap();
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
            || String::from("No request found.\n"),
            |client| {
                let req_line = client.req.request_line();
                let headers = client.req.headers_to_string();
                let headers = headers.trim_end();
                client.req.body.as_ref().map_or_else(
                    || format!("\n{YLW}{req_line}{CLR}\n{headers}\n\n"),
                    |body| {
                        let body = String::from_utf8_lossy(&body);
                        format!(
                            "\n{YLW}{req_line}{CLR}\n{headers}\n\n{}\n\n",
                            body.trim_end()
                        )
                    }
                )
            });

        self.stdout.write_all(output.as_bytes()).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_response_body(&mut self) {
        let output = self.response.as_ref().map_or_else(
            || String::from("No response found.\n"),
            |res| res.body.as_ref().map_or_else(
                || String::from("This response does not have a body."),
                |body| {
                    let body = String::from_utf8_lossy(&body);
                    format!("\n{}\n\n", body.trim_end())
                })
            );

        self.stdout.write_all(output.as_bytes()).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_response(&mut self) {
        let output = self.response.as_ref().map_or_else(
            || String::from("No response found.\n"),
            |res| {
                let status_line = res.status_line();
                let headers = res.headers_to_string();
                let headers = headers.trim_end();
                res.body.as_ref().map_or_else(
                    || format!("\n{PURP}{status_line}{CLR}\n{headers}\n\n"),
                    |body| {
                        let body = String::from_utf8_lossy(&body);
                        format!(
                            "\n{PURP}{status_line}{CLR}\n{headers}\n\n{}\n\n",
                            body.trim_end()
                        )
                    })
            });

        self.stdout.write_all(output.as_bytes()).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_status_line(&mut self) {
        let output = self.response.as_ref().map_or_else(
            || String::from("No status line found.\n"),
            |res| format!("{PURP}{}{CLR}\n\n", res.status_line()),
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

    fn connection_closed(&self) -> bool {
        self.response.as_ref().map_or(false,
            |res| res.has_close_connection_header()
        )
    }
}

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
            "close" | "quit" => break 'outer,
            "clear" => browser.clear_screen(),
            "help" => browser.show_help(),
            "home" => browser.set_home_mode(),
            "normal" => browser.set_output_style(OutputStyle::Normal),
            "quiet" => browser.set_output_style(OutputStyle::Quiet),
            "verbose" => browser.set_output_style(OutputStyle::Verbose),
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
                            "close" | "quit" => break 'outer,
                            "clear" => browser.clear_screen(),
                            "help" => browser.show_help(),
                            "home" => browser.set_home_mode(),
                            "normal" => browser.set_output_style(OutputStyle::Normal),
                            "quiet" => browser.set_output_style(OutputStyle::Quiet),
                            "verbose" => browser.set_output_style(OutputStyle::Verbose),
                            path if path.starts_with('/') => {
                                browser.set_path(path);
                                browser.send();
                                browser.recv();

                                if browser.response.is_some() {
                                    browser.print_output();
                                    browser.in_path_mode = !browser.connection_closed();
                                    browser.response = None;
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

// Output style options.
enum OutputStyle {
    Quiet,
    Normal,
    Verbose,
}

impl fmt::Display for OutputStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quiet => write!(f, "quiet"),
            Self::Normal => write!(f, "normal"),
            Self::Verbose => write!(f, "verbose"),
        }
    }
}

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
            output_style: OutputStyle::Normal,
            in_path_mode: false,
            client: None,
            response: None,
            stdin,
            stdout,
        }
    }

    fn clear_screen(&mut self) {
        self.stdout.write_all(b"\x1b[2J\x1b[1;1H").unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_intro_message(&mut self) {
        writeln!(&mut self.stdout, "{CYAN}rust_browser{CLR} is an HTTP client.").unwrap();
        write!(&mut self.stdout, "Enter a URI at the prompt or ").unwrap();
        writeln!(&mut self.stdout, "try `{YLW}help{CLR}` to see all options.\n").unwrap();
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
            "Output style set to {CYAN}{}{CLR}.",
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
        let help_msg = format!("\n\
            {PURP}HELP:{CLR} The prompt shows which mode you are in.\n\n\
            {GRN}[HOME]${CLR}\n    \
                Enter an HTTP URI to send a request to a remote host.\n\
            {YLW}[REMOTE ADDRESS]${CLR}\n    \
                When you connect to a remote server, its address is displayed in \n    \
                the prompt. Enter relative paths (e.g. \"/my/path\") to request \n    \
                target resources on that server.\n\n\
            {PURP}COMMANDS:{CLR}\n    \
                clear    Clear the terminal.\n    \
                close    Close the program.\n    \
                help     Print this help message.\n    \
                home     Exits {YLW}PATH{CLR} mode and returns to {GRN}HOME{CLR} mode.\n    \
                normal   Only print responses (default).\n    \
                quiet    Only print status lines.\n    \
                verbose  Print full requests and responses.\n\n\
            {PURP}EXAMPLE URI'S:{CLR}\n    \
                httpbin.org/deny\n    \
                http://www.httpbin.org/status/201\n    \
                127.0.0.1:80/my_file.txt\n\
        ");

        writeln!(&mut self.stdout, "{help_msg}").unwrap();
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
            |res| format!("{PURP}{}{CLR}\n", res.status_line()),
        );

        self.stdout.write_all(output.as_bytes()).unwrap();
        self.stdout.flush().unwrap();
    }

    fn print_output(&mut self) {
        match self.output_style {
            OutputStyle::Quiet => {
                self.print_status_line();
            },
            OutputStyle::Normal => {
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

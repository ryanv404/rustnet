use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{self, BufRead, BufWriter, StdoutLock, Write};

use rustnet::{Client, Connection, NetError, NetResult, Request, Response};
use rustnet::colors::*;
use rustnet::util;

// Output styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Style {
    ResBody,
    Request,
    Response,
    StatusLine,
    Verbose,
}

impl Display for Style {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::ResBody => write!(f, "response body"),
            Self::Request => write!(f, "request"),
            Self::Response => write!(f, "response"),
            Self::StatusLine => write!(f, "status line"),
            Self::Verbose => write!(f, "verbose"),
        }
    }
}

pub type Writer<'a> = BufWriter<StdoutLock<'a>>;

// A simple shell-like TUI for an HTTP client.
#[derive(Debug)]
pub struct Tui {
    style: Style,
    running: bool,
    req: Option<Request>,
    res: Option<Response>,
    conn: Option<Connection>,
}

impl Tui {
    fn new() -> Self {
        Self {
            style: Style::Response,
            running: false,
            req: None,
            res: None,
            conn: None
        }
    }

    pub fn run() {
        let mut tui = Tui::new();

        if let Err(e) = tui.start_main_loop() {
            eprintln!("Error: {e}");
        }
    }

    fn start_main_loop(&mut self) -> NetResult<()> {
        let mut stdin = io::stdin().lock();
        let mut out = BufWriter::new(io::stdout().lock());

        self.clear_screen(&mut out)?;
        self.print_intro_msg(&mut out)?;

        self.running = true;
        let mut line = String::new();

        while self.running {
            self.print_prompt(&mut out)?;

            self.req = None;
            self.res = None;
            self.conn = None;

            line.clear();
            stdin.read_line(&mut line)?;
            self.handle_input(line.trim(), &mut out)?;
        }

        out.write_all(b"\n")?;
        out.flush()?;
        Ok(())
    }

    fn handle_input(
        &mut self,
        input: &str,
        out: &mut Writer<'_>
    ) -> NetResult<()> {
        match input {
            "" => {},
            "close" | "quit" => self.running = false,
            "clear" => self.clear_screen(out)?,
            "help" => self.print_help(out)?,
            "body" => self.set_output(Style::ResBody, out)?,
            "request" => self.set_output(Style::Request, out)?,
            "response" => self.set_output(Style::Response, out)?,
            "status" => self.set_output(Style::StatusLine, out)?,
            "verbose" => self.set_output(Style::Verbose, out)?,
            uri => match util::parse_uri(uri).ok() {
                Some((ref addr, ref path)) => {
                    if let Ok(mut client) = Client::builder()
                        .addr(addr)
                        .path(path)
                        .build()
                    {
                        self.req = client.req.take();

                        if self.style != Style::Request {
                            self.conn = Some(client.conn);
                            self.send()?;
                            self.recv()?;
                        }

                        self.print_output(out)?;
                        self.req = None;
                    } else {
                        self.warn_no_connection(out, addr)?;
                    }
                },
                None => self.warn_invalid_input(out, input)?,
            },
        }

        Ok(())
    }

    // Clear the screen and move the cursor to the top left.
    fn clear_screen(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
        out.write_all(b"\x1b[2J\x1b[1;1H")?;
        out.flush()?;
        Ok(())
    }

    fn set_output(&mut self, style: Style, out: &mut Writer<'_>) -> NetResult<()> {
        self.style = style;
        writeln!(out, "Output set to: {YLW}{}{CLR}", &self.style)?;
        out.flush()?;
        Ok(())
    }

    fn print_intro_msg(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
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

		writeln!(out, "{GRN}http_tui/0.1\n\n{PURP}{FACE}{CLR}\n")?;
        out.flush()?;
        Ok(())
    }

    fn warn_invalid_input(
        &mut self,
        out: &mut Writer<'_>,
        input: &str
    ) -> NetResult<()> {
        writeln!(out, "{RED}Invalid input: \"{input}\"{CLR}")?;
        out.flush()?;
        Ok(())
    }

    fn warn_no_connection(
        &mut self,
        out: &mut Writer<'_>,
        addr: &str
    ) -> NetResult<()> {
        writeln!(out, "{RED}Unable to connect to \"{addr}\"{CLR}")?;
        out.flush()?;
        Ok(())
    }

    fn print_prompt(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
        match self.res {
            Some(ref res) => match res.status_code() {
                code @ (100..=199 | 300..=399) => {
                    write!(out, "{CYAN}[{BLU}{code}{CYAN}|ADDRESS]${CLR} ")?;
                },
                code @ 200..=299 => {
                    write!(out, "{CYAN}[{GRN}{code}{CYAN}|ADDRESS]${CLR} ")?;
                },
                code @ 400..=599 => {
                    write!(out, "{CYAN}[{RED}{code}{CYAN}|ADDRESS]${CLR} ")?;
                },
                code => {
                    write!(out, "{CYAN}[{YLW}{code}{CYAN}|ADDRESS]${CLR} ")?;
                },
            },
            None => write!(out, "{CYAN}[ADDRESS]${CLR} ")?,
        }
        out.flush()?;
        Ok(())
    }

    fn print_help(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
        writeln!(out, "\n\
{PURP}http_tui{CLR} is a shell-like HTTP client.\n
{PURP}HELP:{CLR}
    Enter an HTTP URI to send a request.\n
{PURP}COMMANDS:{CLR}
    body      Only print response bodies.
    clear     Clear the terminal.
    close     Close the program.
    help      Print this help message.
    request   Only print requests (but do not send them).
    response  Only print responses (default).
    status    Only print status lines.
    verbose   Print both the requests and the responses.\n")?;
        out.flush()?;
        Ok(())
    }

    fn send(&mut self) -> NetResult<()> {
        match self.req.take() {
            None => Err(NetError::NotConnected),
            Some(mut req) => match self.conn.as_mut() {
                None => Err(NetError::NotConnected),
                Some(conn) => {
                    conn.send_request(&mut req)?;
                    self.req = Some(req);
                    Ok(())
                },
            },
        }
    }

    fn recv(&mut self) -> NetResult<()> {
        match self.conn.as_mut() {
            None => Err(NetError::NotConnected),
            Some(conn) => {
                let res = conn.recv_response()?;
                self.res = Some(res);
                Ok(())
            },
        }
    }

    fn print_request(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
        if let Some(req) = self.req.as_ref() {
            req.request_line.write_color(out)?;
            req.headers.write_color(out)?;

            if req.body.is_printable() {
                writeln!(out, "\n{}", &req.body)?;
            } else if !req.body.is_empty() {
                writeln!(out, "\nBody cannot be printed.")?;
            }
        } else {
            writeln!(out, "No request found.")?;
        }

        writeln!(out)?;
        Ok(())
    }

    fn print_response(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
        match self.res.as_ref() {
            Some(res) => {
                if self.style != Style::ResBody {
                    res.status_line.write_color(out)?;

                    if self.style == Style::StatusLine {
                        return Ok(());
                    }

                    res.headers.write_color(out)?;
                }

                if res.body.is_printable() {
                    writeln!(out, "\n{}", &res.body)?;
                } else if !res.body.is_empty() {
                    writeln!(out, "\nBody cannot be printed.")?;
                }
            },
            None => writeln!(out, "No response found.")?,
        }

        writeln!(out)?;
        Ok(())
    }

    fn print_verbose(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
        self.print_request(out)?;
        self.print_response(out)?;
        Ok(())
    }

    fn print_output(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
        match self.style {
            Style::Verbose => self.print_verbose(out)?,
            Style::Request => self.print_request(out)?,
            Style::StatusLine | Style::ResBody | Style::Response => {
                self.print_response(out)?;
            },
        }

        out.flush()?;
        Ok(())
    }
}

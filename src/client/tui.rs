use std::io::{self, BufRead, BufWriter, StdoutLock, Write};

use crate::{Client, NetResult};
use crate::colors::{CLR, CYAN, GRN, PURP, RED, YLW};
use crate::util;

type Writer<'a> = BufWriter<StdoutLock<'a>>;

// A simple shell-like TUI for an HTTP client.
#[derive(Debug)]
pub struct Tui {
    running: bool,
    do_not_send: bool,
    client: Client,
}

impl Tui {
    fn new() -> Self {
        Self {
            running: false,
            do_not_send: false,
            client: Client::default()
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

        let mut line = String::with_capacity(1024);

        while self.running {
            self.print_prompt(&mut out)?;

            self.client.req = None;
            self.client.res = None;
            self.client.conn = None;

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
            "body" => self.client.output.format_str("b"),
            "request" => {
                self.do_not_send = true;
                self.client.output.format_str("r");
                self.print_output_style("request", out)?;
            },
            "response" => self.client.output.format_str("rhb"),
            "status" => self.client.output.format_str("s"),
            "verbose" => self.client.output.format_str("*"),
            uri => match util::parse_uri(uri).ok() {
                None => self.warn_invalid_input(out, input)?,
                Some((ref addr, ref path)) => {
                    if let Ok(client) = Client::builder()
                        .addr(addr)
                        .path(path)
                        .build()
                    {
                        self.client = client;

                        if !self.do_not_send {
                            self.send()?;
                            self.recv()?;
                        }

                        self.print_output(out)?;

                        self.client.req = None;
                    } else {
                        self.warn_no_connection(out, addr)?;
                    }
                },
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

    fn print_output_style(
        &mut self,
        style: &str,
        out: &mut Writer<'_>
    ) -> NetResult<()> {
        writeln!(out, "Output set to: {YLW}{style}{CLR}")?;
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
        match self.client.res {
            None => write!(out, "{CYAN}[ADDRESS]${CLR} ")?,
            Some(ref res) => {
                let (color, code) = match res.status_code() {
                    code @ 200..=299 => (GRN, code),
                    code @ 400..=599 => (RED, code),
                    code => (YLW, code),
                };

                write!(out, "{CYAN}[{color}{code}{CYAN}|ADDRESS]${CLR} ")?;
            },
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
    verbose   Print both requests and responses.\n")?;

        out.flush()?;
        Ok(())
    }

    fn send(&mut self) -> NetResult<()> {
        self.client.send_request()
    }

    fn recv(&mut self) -> NetResult<()> {
        self.client.recv_response()
    }

    fn print_output(&mut self, out: &mut Writer<'_>) -> NetResult<()> {
        self.client.print(out)?;
        out.flush()?;
        Ok(())
    }
}

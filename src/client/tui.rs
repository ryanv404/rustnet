use std::io::{BufRead, BufWriter, StdoutLock, Write, stdin, stdout};
use std::process;

use crate::{Client, NetResult};
use crate::colors::{CLR, CYAN, GRN, PURP, RED, YLW};
use crate::util;

type Writer<'out> = BufWriter<StdoutLock<'out>>;

// A simple shell-like TUI for an HTTP client.
#[derive(Debug)]
pub struct Tui {
    running: bool,
    do_send: bool,
    client: Client,
}

impl Tui {
    fn new() -> Self {
        Self {
            running: false,
            do_send: true,
            client: Client::default()
        }
    }

    pub fn run() {
        let mut tui = Self::new();

        if let Err(e) = tui.start_main_loop() {
            eprintln!("Error: {e}");
            process::exit(1);
        }

        process::exit(0);
    }

    fn start_main_loop(&mut self) -> NetResult<()> {
        let mut writer = BufWriter::new(stdout().lock());

        Self::clear_screen(&mut writer)?;
        Self::print_intro_msg(&mut writer)?;

        let mut line = String::with_capacity(1024);

        self.running = true;

        while self.running {
            line.clear();

            self.client.req = None;
            self.client.res = None;
            self.client.conn = None;

            self.print_prompt(&mut writer)?;

            stdin().lock().read_line(&mut line)?;

            self.handle_input(&mut writer, line.trim())?;
        }

        writer.write_all(b"\n")?;

        writer.flush()?;

        Ok(())
    }

    fn handle_input(
        &mut self,
        writer: &mut Writer<'_>,
        input: &str
    ) -> NetResult<()> {
        match input {
            "" => {},
            "close" | "quit" => self.running = false,
            "clear" => Self::clear_screen(writer)?,
            "help" => Self::print_help(writer)?,
            "body" => self.client.output.format_str("b"),
            "request" => {
                self.do_send = false;
                self.client.output.format_str("r");
                Self::print_output_style(writer, "request")?;
            },
            "response" => self.client.output.format_str("rhb"),
            "status" => self.client.output.format_str("s"),
            "verbose" => self.client.output.format_str("*"),
            uri => match util::parse_uri(uri).ok() {
                None => Self::warn_invalid_input(writer, input)?,
                Some((ref addr, ref path)) => {
                    if let Ok(client) = Client::builder()
                        .addr(addr)
                        .path(path)
                        .build()
                    {
                        self.client = client;

                        if self.do_send {
                            self.send()?;
                            self.recv()?;
                        }

                        self.print_output(writer)?;

                        self.client.req = None;
                    } else {
                        Self::warn_no_connection(writer, addr)?;
                    }
                },
            },
        }

        Ok(())
    }

    // Clear the screen and move the cursor to the top left.
    fn clear_screen(writer: &mut Writer<'_>) -> NetResult<()> {
        writer.write_all(b"\x1b[2J\x1b[1;1H")?;
        writer.flush()?;
        Ok(())
    }

    fn print_output_style(
        writer: &mut Writer<'_>,
        style: &str
    ) -> NetResult<()> {
        writeln!(writer, "Output set to: {YLW}{style}{CLR}")?;
        writer.flush()?;
        Ok(())
    }

    fn print_intro_msg(writer: &mut Writer<'_>) -> NetResult<()> {
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

		writeln!(writer, "{GRN}http_tui/0.1\n\n{PURP}{FACE}{CLR}\n")?;
        writer.flush()?;
        Ok(())
    }

    fn warn_invalid_input(
        writer: &mut Writer<'_>,
        input: &str
    ) -> NetResult<()> {
        writeln!(writer, "{RED}Invalid input: \"{input}\"{CLR}")?;
        writer.flush()?;
        Ok(())
    }

    fn warn_no_connection(
        writer: &mut Writer<'_>,
        addr: &str
    ) -> NetResult<()> {
        writeln!(writer, "{RED}Unable to connect to \"{addr}\"{CLR}")?;
        writer.flush()?;
        Ok(())
    }

    fn print_prompt(&mut self, writer: &mut Writer<'_>) -> NetResult<()> {
        match self.client.res {
            None => write!(writer, "{CYAN}[ADDRESS]${CLR} ")?,
            Some(ref res) => {
                let (color, code) = match res.status_code() {
                    code @ 200..=299 => (GRN, code),
                    code @ 400..=599 => (RED, code),
                    code => (YLW, code),
                };

                write!(writer, "{CYAN}[{color}{code}{CYAN}|ADDRESS]${CLR} ")?;
            },
        }

        writer.flush()?;
        Ok(())
    }

    fn print_help(writer: &mut Writer<'_>) -> NetResult<()> {
        writeln!(writer, "\n\
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

        writer.flush()?;
        Ok(())
    }

    fn send(&mut self) -> NetResult<()> {
        self.client.send_request()
    }

    fn recv(&mut self) -> NetResult<()> {
        self.client.recv_response()
    }

    fn print_output(&mut self, writer: &mut Writer<'_>) -> NetResult<()> {
        self.client.print(writer)?;
        writer.flush()?;
        Ok(())
    }
}

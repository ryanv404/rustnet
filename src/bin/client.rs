use std::env;

use rustnet::{Client, ClientCli, Method, NetResult};

mod client_tui;
use client_tui::Browser;

const RED: &str = "\x1b[91m";
const GRN: &str = "\x1b[92m";
const PURP: &str = "\x1b[95m";
const CLR: &str = "\x1b[0m";

#[rustfmt::skip]
fn main() -> NetResult<()> {
    // Handle CLI arguments.
    let cli = ClientCli::parse(env::args());

    // Start TUI if selected.
    if cli.tui {
        return Browser::run_client_tui()
    }

    // Parse the URI argument.
    let (addr, path) = Client::parse_uri(cli.uri)?;

    // Print request without sending.
    if cli.request {
        todo!();
        return;
    }

    // Create a client and send a request.
    let mut client = Client::builder()
        .addr(&addr)
        .method(cli.method)
        .path(cli.path.as_ref().unwrap_or(&path))
        .send()?;

    // Receive response.
    client.recv();

    // Ignore Date headers.
    if cli.no_dates {
        client.remove_date_headers();
    }

    let mut w = io::stdout().lock().unwrap();

    if let Some(req) = client.req.as_ref() {
        if cli.output_req_line {
            writeln!(&mut w, "{}", req.request_line.to_string_plain())?;
        }

        if cli.output_req_headers {
            writeln!(&mut w, "{}", req.headers.to_string_plain())?;
        }

        if cli.output_req_body && req.body.is_printable() {
            writeln!(&mut w, "{}", &req.body)?;
        }
    }

    if let Some(res) = client.res.as_ref() {
        if cli.output_status_line {
            writeln!(&mut w, "{}", req.status_line.to_string_plain())?;
        }

        if cli.output_res_headers {
            writeln!(&mut w, "{}", res.headers.to_string_plain())?;
        }

        if cli.output_res_body && res.body.is_printable() {
            writeln!(&mut w, "{}", &res.body)?;
        }
    }

    w.flush()?;
    Ok(())
}

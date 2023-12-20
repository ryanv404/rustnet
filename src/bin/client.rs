use std::env;
use std::io::{stdout, Write};

use rustnet::{Client, ClientCli, NetResult};

mod tui;
use tui::Tui;

#[rustfmt::skip]
fn main() -> NetResult<()> {
    // Handle CLI arguments.
    let cli = ClientCli::parse_args(env::args())?;

    // Start TUI if selected.
    if cli.tui {
        Tui::run();
        return Ok(());
    }

    // Create an HTTP client.
    let mut client = Client::builder()
        .method(cli.method)
        .addr(&cli.addr)
        .path(&cli.path)
        .build()?;

    // Apply any command line headers.
    if !cli.headers.is_empty() {
        todo!();
    }

    // Send request and receive response.
    if cli.do_send {
        client.send()?;
        client.recv();
    }

    // Ignore Date headers.
    if cli.no_dates {
        client.remove_date_headers();
    }

    let mut w = stdout().lock();

    // Handle request output.
    if let Some(req) = client.req.as_ref() {
        match (cli.out_req_line, cli.use_color) {
            (true, true) => {
                w.write_all(req.request_line.to_color_string().as_bytes())?;
            },
            (true, false) => {
                w.write_all(req.request_line.to_plain_string().as_bytes())?;
            },
            (_, _) => {},
        }

        match (cli.out_req_headers, cli.use_color) {
            (true, true) => {
                w.write_all(req.headers.to_color_string().as_bytes())?;
            },
            (true, false) => {
                w.write_all(req.headers.to_plain_string().as_bytes())?;
            },
            (_, _) => {},
        }

        if cli.out_req_body && req.body.is_printable() {
            if cli.out_req_headers {
                w.write_all(b"\n")?;
            }

            w.write_all(req.body.as_bytes())?;
            w.write_all(b"\n")?;
        }
    }

    if needs_a_newline(&cli) {
        w.write_all(b"\n")?;
    }

    // Handle response output.
    if let Some(res) = client.res.as_ref() {
        match (cli.out_status_line, cli.use_color) {
            (true, true) => {
                w.write_all(res.status_line.to_color_string().as_bytes())?;
            },
            (true, false) => {
                w.write_all(res.status_line.to_plain_string().as_bytes())?;
            },
            (_, _) => {},
        }

        match (cli.out_res_headers, cli.use_color) {
            (true, true) => {
                w.write_all(res.headers.to_color_string().as_bytes())?;
            },
            (true, false) => {
                w.write_all(res.headers.to_plain_string().as_bytes())?;
            },
            (_, _) => {},
        }

        if cli.out_res_body && res.body.is_printable() {
            if cli.out_res_headers {
                w.write_all(b"\n")?;
            }

            w.write_all(res.body.as_bytes())?;
            w.write_all(b"\n")?;
        }
    }

    w.write_all(b"\n")?;
    w.flush()?;
    Ok(())
}

fn needs_a_newline(cli: &ClientCli) -> bool {
    // A request component is output.
    if cli.out_req_line || cli.out_req_headers || cli.out_req_body {
        // And a response component is output.
        if cli.out_status_line || cli.out_res_headers || cli.out_res_body {
            return true;
        }
    }

    false
}

use std::env;
use std::io::{BufWriter, Write, stdout};

use rustnet::{Client, ClientCli, NetResult};
use rustnet::colors::{CLR, RED};

mod tui;
use tui::Tui;

fn main() -> NetResult<()> {
    // Handle CLI arguments.
    let mut cli = ClientCli::parse_args(env::args());

    // Start TUI if selected.
    if cli.run_tui {
        Tui::run();
        return Ok(());
    }

    // Create an HTTP client.
    let builder = Client::builder()
        .addr(&cli.addr)
        .path(&cli.path)
        .method(&cli.method)
        .headers(&cli.headers)
        .body(&cli.body);

    if cli.debug {
        dbg!(&builder);
        return Ok(());
    }

    if cli.do_not_send {
        let mut client = builder.build()?;
        handle_output(&mut client, &mut cli)?;
        return Ok(());
    }

    match builder.send() {
        Ok(mut client) => {
            client.recv()?;
            handle_output(&mut client, &mut cli)
        },
        Err(e) => {
            eprintln!("{RED}Unable to connect to `{}`.\n{e}{CLR}", &cli.addr);
            Err(e)
        },
    }
}

fn handle_output(
    client: &mut Client,
    cli: &mut ClientCli
) -> NetResult<()> {
    // Ignore Date headers.
    if cli.output.no_dates {
        client.remove_date_headers();
    }

    let mut is_head_route = false;
    let mut w = BufWriter::new(stdout().lock());

    // Handle request output.
    if let Some(req) = client.req.as_ref() {
        cli.output.write_request(req, &mut w)?;

        is_head_route = req.route().is_head();
    }

    // Exit if the "--request" option was provided.
    if cli.do_not_send {
        return Ok(());
    }

    // Handle response output.
    if let Some(res) = client.res.as_ref() {
        let do_separator = cli.output.include_separator();

        cli.output.write_response(
            do_separator,
            is_head_route,
            res,
            &mut w
        )?;
    }

    writeln!(&mut w)?;
    w.flush()?;
    Ok(())
}

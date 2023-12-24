use std::env;
use std::io::{BufWriter, stdout};

use rustnet::{Client, ClientCli, NetResult};

fn main() -> NetResult<()> {
    // Handle CLI arguments.
    let cli = ClientCli::parse_args(&mut env::args())?;

    // Create an HTTP client.
    let builder = Client::builder()
        .output(&cli.output)
        .addr(&cli.addr)
        .path(&cli.path)
        .method(&cli.method)
        .headers(&cli.headers)
        .body(&cli.body);

    if cli.debug {
        dbg!(&builder);
        return Ok(());
    }

    let mut out = BufWriter::new(stdout().lock());

    if cli.do_not_send {
        builder
            .build()
            .and_then(|mut client| client.print(&mut out))?;

        return Ok(());
    }

    builder
        .send()
        .and_then(|mut client| {
            client.recv_response()?;
            client.print(&mut out)?;
            Ok(())
        })
}

use std::env;
use std::io::{BufWriter, stdout};

use rustnet::{Client, ClientCli, NetResult};

fn main() -> NetResult<()> {
    // Handle CLI arguments.
    let cli = ClientCli::parse_args(env::args())?;

    // Create an HTTP client.
    let builder = Client::builder()
        .addr(&cli.addr)
        .path(&cli.path)
        .method(&cli.method)
        .headers(&cli.headers)
        .body(&cli.body)
        .output(&cli.output);

    if cli.debug {
        dbg!(&builder);
        return Ok(());
    }

    let mut out = BufWriter::new(stdout().lock());

    if cli.do_not_send {
        let mut client = builder.build()?;
        client.print(&mut out)?;
        return Ok(());
    }

    match builder.send() {
        Ok(mut client) => {
            client.recv_response()?;
            client.print(&mut out)?;
            Ok(())
        },
        Err(e) => Err(e),
    }
}

use std::env;
use std::io::{BufWriter, stdout};

use rustnet::{Client, ClientCli};

fn main() {
    let cli = ClientCli::parse_args(&mut env::args());

    let mut builder = Client::builder();
    builder
        .addr(&cli.addr)
        .path(&cli.path)
        .method(cli.method.clone())
        .headers(cli.headers.clone())
        .body(cli.body.clone())
        .output(cli.output);

    if cli.debug {
        dbg!(&builder);
        return;
    }

    let mut client = match builder.build() {
        Ok(client) => client,
        Err(ref e) => {
            eprintln!("Error while building client.\n{e}");
            return;
        },
    };

    if cli.do_send {
        if let Err(ref e) = client.send_request() {
            eprintln!("Error while sending request.\n{e}");
            return;
        }

        if let Err(ref e) = client.recv_response() {
            eprintln!("Error while receiving response.\n{e}");
            return;
        }
    }

    let mut writer = BufWriter::new(stdout().lock());

    if let Err(ref e) = client.print(&mut writer) {
        eprintln!("Error while handling client output.\n{e}");
    }
}

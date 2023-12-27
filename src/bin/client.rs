use std::env;
use std::io::{BufWriter, stdout};

use rustnet::{Client, ClientCli};
use rustnet::header_name::HOST;

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
    } else if let Some(req) = client.req.as_mut() {
        // Ensure Host headers is set.
        if !req.headers.contains(&HOST) {
            if let Some(conn) = client.conn.as_mut() {
                let stream = conn.writer.get_ref();

                if let Ok(remote) = stream.peer_addr() {
                    req.headers.host(remote);
                }
            }
        }
    }

    let mut writer = BufWriter::new(stdout().lock());

    if let Err(ref e) = client.print(&mut writer) {
        eprintln!("Error while handling client output.\n{e}");
    }
}

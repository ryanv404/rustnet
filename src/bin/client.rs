use std::collections::VecDeque;
use std::env;
use std::io::{BufWriter, stdout};

use rustnet::ClientCli;
use rustnet::header_name::HOST;

fn main() {
    let args = env::args().collect::<VecDeque<String>>();

    let mut args = args
        .iter()
        .map(|s| s.as_ref())
        .collect::<VecDeque<&str>>();

    let mut client = match ClientCli::parse_args(&mut args) {
        Ok(ref client) if client.debug => {
            dbg!(client);
            return;
        },
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error while building client.\n{e}");
            return;
        },
    };

    if client.do_send {
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

use std::collections::VecDeque;
use std::env;

use rustnet::{Client, ClientCli};

fn main() {
    let args = env::args().collect::<VecDeque<String>>();

    let mut args = args
        .iter()
        .map(|s| s.as_ref())
        .collect::<VecDeque<&str>>();

    let Ok(mut client) = ClientCli::parse_args(&mut args)
        .and_then(Client::try_from)
    else {
        eprintln!("Unable to build the client.");
        return;
    };

    if client.do_debug {
        println!("{client:#?}");
        return;
    }

    if client.do_send {
        if let Err(ref e) = client.send_request() {
            eprintln!("Error while sending request.\n{e}");
            return;
        }

        if let Err(ref e) = client.recv_response() {
            eprintln!("Error while receiving response.\n{e}");
            return;
        }
    } 

    client.print();
}

use std::collections::VecDeque;
use std::env;

mod cli;
mod tui;

use cli::ClientCli;
use tui::Tui;

fn main() {
    let args = env::args().collect::<VecDeque<String>>();

    let mut args = args
        .iter()
        .map(|s| s.as_ref())
        .collect::<VecDeque<&str>>();

    let Ok(mut client) = ClientCli::parse_args(&mut args) else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rustnet::{
        Body, Client, Headers, Method, Style, StyleKind, StyleParts, utils,
    };

    #[test]
    fn parse_args() {
        let mut args = VecDeque::from([
            "./client",
            "--plain",
            "--no-dates",
            "--output", "/Bs2Rhb1",
            "--method", "posT",
            "-H", "acCEpT:*/*",
            "-H", "conteNt-leNgth:13",
            "-H", "caCHe-controL:no-CACHE",
            "--debug",
            "-H", "cOntent-tYpe:text/html; charset=utf-8",
            "-H", "pineaPPle:yUm123",
            "--body", "This is a test meSSage :) in the request bOdy.",
            "httpbin.org/json"
        ]);

        let mut test_client = ClientCli::parse_args(&mut args).unwrap();

        let style = Style {
            req: StyleKind::Plain(StyleParts::LineBody),
            res: StyleKind::Plain(StyleParts::All)
        };

        let mut headers = Headers::new();
        headers.add_accept("*/*");
        headers.add_content_length(13);
        headers.add_cache_control("no-cache");
        headers.add_content_type("text/html; charset=utf-8");

        let custom_name = utils::to_titlecase(b"Pineapple");
        headers.insert(custom_name.into(), "yum123".into());

        let body = Body::Text(
            "This is a test meSSage :) in the request bOdy.".into()
        );

        let expected_cli = ClientCli {
            no_dates: true,
            do_debug: true,
            do_plain: true,
            style,
            method: Method::Post,
            path: "/json".into(),
            addr: Some("httpbin.org:80".to_string()),
            headers,
            body,
            ..ClientCli::default()
        };

        let mut expected_client = Client::try_from(expected_cli).unwrap();

        if let Some(req) = test_client.req.as_mut() {
            req.headers.header("Host", "httpbin.org");
        }

        if let Some(req) = expected_client.req.as_mut() {
            req.headers.header("Host", "httpbin.org");
        }

        assert_eq!(test_client, expected_client);
    }
}

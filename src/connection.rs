use std::{
    io::{self, BufReader, BufWriter},
    net::{IpAddr, TcpStream}
};

#[derive(Debug)]
pub struct Connection {
    pub remote_ip: IpAddr,
    pub remote_port: u16,
    pub reader: BufReader<TcpStream>,
    pub writer: BufWriter<TcpStream>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        let sock = stream.peer_addr()?;
        let remote_ip = sock.ip();
        let remote_port = sock.port();

        let clone = stream.try_clone()?;
        let reader = BufReader::new(stream);
        let writer = BufWriter::new(clone);

        Ok(Self { remote_ip, remote_port, reader, writer })
    }

    pub fn remote_ip(&self) -> &IpAddr {
        &self.remote_ip
    }

    pub fn remote_port(&self) -> u16 {
        self.remote_port
    }
}

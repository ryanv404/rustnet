use std::io::{BufRead, BufReader, BufWriter, Error as IoError, Read, Result as IoResult, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::Mutex;

use crate::consts::{READER_BUFSIZE, WRITER_BUFSIZE};

#[derive(Debug)]
pub struct NetReader(pub BufReader<TcpStream>);

impl From<TcpStream> for NetReader {
    fn from(stream: TcpStream) -> Self {
        Self(BufReader::with_capacity(READER_BUFSIZE, stream))
    }
}

impl Read for NetReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.0.read(buf)
    }
}

impl BufRead for NetReader {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.0.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.0.consume(amt);
    }
}

#[derive(Debug)]
pub struct NetWriter(pub Mutex<BufWriter<TcpStream>>);

impl From<TcpStream> for NetWriter {
    fn from(stream: TcpStream) -> Self {
        let writer = BufWriter::with_capacity(WRITER_BUFSIZE, stream);
        Self(Mutex::new(writer))
    }
}

impl Write for NetWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.0.lock().unwrap().flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.0.lock().unwrap().write_all(buf)
    }
}

/// Represents a TCP connection to a remote client.
#[derive(Debug)]
pub struct Connection {
    /// The local socket address.
    pub local_addr: SocketAddr,
    /// The remote socket address.
    pub remote_addr: SocketAddr,
    /// Reads requests from the TCP connection.
    pub reader: NetReader,
    /// Writes responses to the TCP connection.
    pub writer: NetWriter,
}

impl TryFrom<(TcpStream, SocketAddr)> for Connection {
    type Error = IoError;

    fn try_from((stream, addr): (TcpStream, SocketAddr)) -> IoResult<Self> {
        Self::new(stream, addr)
    }
}

impl TryFrom<TcpStream> for Connection {
    type Error = IoError;

    fn try_from(stream: TcpStream) -> IoResult<Self> {
        let addr = stream.peer_addr()?;
        Self::new(stream, addr)
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.writer.write_all(buf)
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.reader.read(buf)
    }
}

impl BufRead for Connection {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt);
    }
}

impl Connection {
    /// Creates a new readable and writable `Connection` instance.
    pub fn new(stream: TcpStream, remote_addr: SocketAddr) -> IoResult<Self> {
        let local_addr = stream.local_addr()?;
        let (r, w) = (stream.try_clone()?, stream);
        let (reader, writer) = (NetReader::from(r), NetWriter::from(w));
        Ok(Self {
            local_addr,
            remote_addr,
            reader,
            writer,
        })
    }

    /// Returns the local client's IP address.
    pub const fn local_ip(&self) -> IpAddr {
        self.local_addr.ip()
    }

    /// Returns the local client's port.
    pub const fn local_port(&self) -> u16 {
        self.local_addr.port()
    }

    /// Returns the remote host's IP address.
    pub const fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
    }

    /// Returns the remote host's port.
    pub const fn remote_port(&self) -> u16 {
        self.remote_addr.port()
    }
}

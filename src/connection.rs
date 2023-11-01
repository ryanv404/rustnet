use std::{
    io::{BufRead, BufReader, BufWriter, Error as IoError, Read, Result as IoResult, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    sync::Mutex,
};

use crate::{
    consts::{READER_BUFSIZE, WRITER_BUFSIZE},
    Request,
};

pub enum Message {
    NewRequest(Request),
    Error(IoError),
}

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
}

/// Represents a TCP connection to a remote client.
pub struct RemoteClient {
    /// The remote connection's socket address.
    pub remote_addr: SocketAddr,
    /// Reads requests from the TCP connection.
    pub reader: NetReader,
    /// Writes responses to the TCP connection.
    pub writer: NetWriter,
}

impl TryFrom<(TcpStream, SocketAddr)> for RemoteClient {
    type Error = IoError;

    fn try_from((stream, addr): (TcpStream, SocketAddr)) -> IoResult<Self> {
        Self::new(stream, addr)
    }
}

impl Write for RemoteClient {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.writer.flush()
    }
}

impl Read for RemoteClient {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.reader.read(buf)
    }
}

impl BufRead for RemoteClient {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt);
    }
}

impl RemoteClient {
    /// Creates a new readable and writable `RemoteClient` instance.
    pub fn new(stream: TcpStream, remote_addr: SocketAddr) -> IoResult<Self> {
        let (r, w) = (stream.try_clone()?, stream);
        let (reader, writer) = (NetReader::from(r), NetWriter::from(w));
        Ok(Self {
            remote_addr,
            reader,
            writer,
        })
    }

    /// Returns the remote client's IP address.
    pub const fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
    }

    /// Returns the remote client's port.
    pub const fn remote_port(&self) -> u16 {
        self.remote_addr.port()
    }

    /// Attempts to clone a new `RemoteClient` instance.
    pub fn try_clone(&self) -> IoResult<Self> {
        let stream = self.reader.0.get_ref().try_clone()?;
        let addr = self.remote_addr;
        Self::new(stream, addr)
    }
}

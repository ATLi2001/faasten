use std::net::TcpStream;
use log::debug;

use crate::syscalls;
use syscalls::syscall::Syscall as SC;
use crate::request;


#[derive(Debug)]
pub enum Error {
    TcpConnectionError,
    TcpIOError(std::io::Error),
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::TcpIOError(e)
    }
}

struct DbServerManager {
    address: String,
}

impl r2d2::ManageConnection for DbServerManager {
    type Connection = TcpStream;
    type Error = std::io::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Ok(TcpStream::connect(&self.address)?)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        let req = request::Request {
            function: String::from("ping"),
            payload: serde_json::Value::Null,
        };
        request::write_u8(&req.to_vec(), conn)?;
        request::read_u8(conn)?;
        Ok(())
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.take_error().ok().flatten().is_some()
    }
}

pub struct DbClient {
    address: String,
    // conn: r2d2::Pool<DbServerManager>,
    stream: TcpStream,
}

impl DbClient {
    pub fn new(address: String) -> Self {
        debug!("db_client created, server at {}", address.clone());
        // let conn = r2d2::Pool::builder().max_size(10).build(DbServerManager { address: address.clone() }).expect("pool");
        let stream = TcpStream::connect(address.clone()).expect("tcpstream");

        DbClient { address: address.clone(), stream }
    }

    /// read key
    pub fn get(&mut self, key: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::ReadKey(syscalls::ReadKey {key});
        // let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        // self.send_sc_get_response(sc, conn)
        self.send_sc_get_response(sc)
    }
    
    /// write key
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::WriteKey(syscalls::WriteKey {key, value});
        // let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        // self.send_sc_get_response(sc, conn)
        self.send_sc_get_response(sc)
    }

    pub fn scan(&mut self, dir: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::ReadDir(syscalls::ReadDir {dir});
        // let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        // self.send_sc_get_response(sc, conn)
        self.send_sc_get_response(sc)
    }

    /// helpers
    fn send_sc_get_response(&mut self, sc: SC) -> Result<Vec<u8>, Error>  {
        use std::io::{Read, Write};

        let mut buf = Vec::new();
        buf.reserve(sc.encoded_len());
        sc.encode(&mut buf);

        // write to server address
        self.stream.write_all(&(buf.len() as u32).to_be_bytes())?;
        self.stream.write_all(buf.as_ref())?;
        // read the response
        let mut lenbuf = [0;4];
        self.stream.read_exact(&mut lenbuf)?;
        let size = u32::from_be_bytes(lenbuf);
        let mut result = vec![0u8; size as usize];
        self.stream.read_exact(&mut result)?;

        Ok(result)
    }
    
}
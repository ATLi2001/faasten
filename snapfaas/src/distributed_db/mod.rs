use std::net::TcpStream;

// use prost::Message;

use crate::syscalls;
use syscalls::syscall::Syscall as SC;

pub mod db_server;
// use self::db_server::DbServer;

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

//////////////
//   APIs   //
//////////////
/// read key
pub fn read_key(db_addr: String, key: Vec<u8>) -> Result<Vec<u8>, Error> {
    let sc = SC::ReadKey(syscalls::ReadKey {key});
    let mut buf = Vec::new();
    buf.reserve(sc.encoded_len());
    sc.encode(&mut buf);
    send_buf_get_response(db_addr, buf)
}

/// write key
pub fn write_key(db_addr: String, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
    let sc = SC::WriteKey(syscalls::WriteKey {key, value});
    let mut buf = Vec::new();
    buf.reserve(sc.encoded_len());
    sc.encode(&mut buf);
    send_buf_get_response(db_addr, buf)
}

/// helpers
fn send_buf_get_response(db_addr: String, buf: Vec<u8>) -> Result<Vec<u8>, Error>  {
    use std::io::{Read, Write};

    match TcpStream::connect(db_addr.clone()) {
      Ok(mut stream) => {
          // write to server address
          stream.write_all(&(buf.len() as u32).to_be_bytes())?;
          stream.write_all(buf.as_ref())?;
  
          // read the response
          let mut lenbuf = [0;4];
          stream.read_exact(&mut lenbuf)?;
          let size = u32::from_be_bytes(lenbuf);
          let mut result = vec![0u8; size as usize];
          stream.read_exact(&mut result)?;
  
          Ok(result)
      },
      Err(_) => {
          Err(Error::TcpConnectionError)
      }
  }
}
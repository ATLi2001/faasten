use std::net::TcpStream;

// use prost::Message;

use crate::syscalls;
use crate::dclabel_helper::dc_label_to_proto_label;
use syscalls::syscall::Syscall as SC;
use labeled::dclabel::DCLabel;

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
    send_sc_get_response(db_addr, sc)
}

/// write key
pub fn write_key(db_addr: String, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
    let sc = SC::WriteKey(syscalls::WriteKey {key, value});
    send_sc_get_response(db_addr, sc)
}

/// read dir
pub fn read_dir(db_addr: String, dir: Vec<u8>) -> Result<Vec<u8>, Error> {
    let sc = SC::ReadDir(syscalls::ReadDir {dir});
    send_sc_get_response(db_addr, sc)
}

pub fn fs_read(db_addr: String, path: &str, cur_label: DCLabel) -> Result<Vec<u8>, Error> {
    let sc = SC::FsRead(syscalls::FsRead {path: path.to_string(), label: Some(dc_label_to_proto_label(&cur_label))});
    send_sc_get_response(db_addr, sc)
}

/// helpers
fn send_sc_get_response(db_addr: String, sc: SC) -> Result<Vec<u8>, Error>  {
    use std::io::{Read, Write};

    let mut buf = Vec::new();
    buf.reserve(sc.encoded_len());
    sc.encode(&mut buf);

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
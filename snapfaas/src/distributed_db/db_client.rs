use std::net::TcpStream;
use log::debug;
use lmdb::WriteFlags;

use crate::syscalls;
use syscalls::syscall::Syscall as SC;
use crate::distributed_db::{DbService, Error};

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
        let sc = SC::Invoke(
            syscalls::Invoke {function: String::from("ping"), payload: String::from("")}
        );
        let result = send_sc_get_response(sc, conn);
        if result.is_ok() {
            Ok(())
        }
        else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "r2d2 connection not valid"))
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.take_error().ok().flatten().is_some()
    }
}

pub struct DbClient {
    address: String,
    conn: r2d2::Pool<DbServerManager>,
}

impl DbService for DbClient {
    fn get(&self, key: Vec<u8>) -> Result<Vec<u8>, Error> {
        debug!("get called");
        let sc = SC::ReadKey(syscalls::ReadKey {key});
        let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        send_sc_get_response(sc, conn)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        debug!("put called");
        let sc = SC::WriteKey(syscalls::WriteKey {key, value, flags: None});
        let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        send_sc_get_response(sc, conn)
    }

    fn add(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::WriteKey(syscalls::WriteKey {key, value, flags: Some(WriteFlags::NO_OVERWRITE.bits())});
        let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        send_sc_get_response(sc, conn)
    }

    fn cas(&self, key: Vec<u8>, expected: Option<Vec<u8>>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::CompareAndSwap(syscalls::CompareAndSwap {key, expected, value});
        let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        send_sc_get_response(sc, conn)
    }
    
    fn scan(&self, dir: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::ReadDir(syscalls::ReadDir {dir});
        let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        send_sc_get_response(sc, conn)
    }
}

impl DbClient {
    pub fn new(address: String) -> Self {
        debug!("db_client created, server at {}", address.clone());
        let conn = r2d2::Pool::builder().max_size(10).build(DbServerManager { address: address.clone() }).expect("pool");

        DbClient { address: address.clone(), conn }
    }
}

// helpers
fn send_sc_get_response(sc: SC, stream: &mut TcpStream) -> Result<Vec<u8>, Error>  {
    use std::io::{Read, Write};

    debug!("send sc, local: {}, peer: {}", stream.local_addr().unwrap(), stream.peer_addr().unwrap());

    let mut buf = Vec::new();
    buf.reserve(sc.encoded_len());
    sc.encode(&mut buf);

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
}
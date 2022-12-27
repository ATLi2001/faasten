use std::net::TcpStream;
use log::debug;
use lmdb::WriteFlags;

use crate::syscalls;
use syscalls::syscall::Syscall as SC;
use crate::distributed_db::{DbService, Error, CACHE_ADDRESS};
use crate::fs::BackingStore;
use prost::Message;

#[derive(Debug)]
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

#[derive(Debug, Clone)]
pub struct DbClient {
    // address: String,
    cache: r2d2::Pool<DbServerManager>,
    conn: r2d2::Pool<DbServerManager>,
}

// legacy for read key, write key, read dir, cas basic operations
impl DbService for DbClient {
    fn get(&self, key: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::ReadKey(syscalls::ReadKey {key});
        let conn = &mut self.conn.get().map_err(|_| Error::TcpConnectionError)?;
        send_sc_get_response(sc, conn)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
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

impl BackingStore for DbClient {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let sc = SC::ReadKey(syscalls::ReadKey {key: Vec::from(key)});
        let conn = &mut self.conn.get().unwrap();
        let resp = send_sc_get_response(sc, conn);
        if resp.is_err() {
            None
        }
        else {
            syscalls::ReadKeyResponse::decode(resp.unwrap().as_ref()).unwrap().value
        }
    }

    fn put(&self, key: &[u8], value: &[u8]) {
        let sc = SC::WriteKey(syscalls::WriteKey {
            key: Vec::from(key), 
            value: Vec::from(value),
            flags: None,
        });
        let cache_conn = &mut self.cache.get().unwrap();
        let _ = send_sc_get_response(sc.clone(), cache_conn);
        let conn = &mut self.conn.get().unwrap();
        let _ = send_sc_get_response(sc, conn);
    }

    fn add(&self, key: &[u8], value: &[u8]) -> bool {
        let sc = SC::WriteKey(syscalls::WriteKey {
            key: Vec::from(key), 
            value: Vec::from(value),
            flags: Some(WriteFlags::NO_OVERWRITE.bits()),
        });
        let cache_conn = &mut self.cache.get().unwrap();
        let _ = send_sc_get_response(sc.clone(), cache_conn);
        let conn = &mut self.conn.get().unwrap();
        let resp = send_sc_get_response(sc, conn);
        if resp.is_err() {
            false
        }
        else {
            syscalls::WriteKeyResponse::decode(resp.unwrap().as_ref()).unwrap().success
        }
    }

    fn cas(&self, key: &[u8], expected: Option<&[u8]>, value: &[u8]) -> Result<(), Option<Vec<u8>>> {
        let mut exp = None; 
        if expected.is_some() {
            exp = Some(Vec::from(expected.unwrap()));
        }
        let sc = SC::CompareAndSwap(syscalls::CompareAndSwap {
            key: Vec::from(key), 
            expected: exp, 
            value: Vec::from(value),
        });
        let cache_conn = &mut self.cache.get().unwrap();
        let _ = send_sc_get_response(sc.clone(), cache_conn);
        let conn = &mut self.conn.get().unwrap();
        let resp = send_sc_get_response(sc, conn);
        let cas_res = syscalls::CompareAndSwapResponse::decode(resp.unwrap().as_ref()).unwrap();
        if cas_res.success {
            Ok(())
        }
        else {
            Err(cas_res.old)
        }
    }
}

impl DbClient {
    pub fn new(address: String) -> Self {
        debug!("db_client created, server at {}", address.clone());
        let cache = r2d2::Pool::builder().max_size(10).build(DbServerManager { address: CACHE_ADDRESS.to_string() }).expect("cache pool");
        let conn = r2d2::Pool::builder().max_size(10).build(DbServerManager { address: address.clone() }).expect("pool");

        DbClient {cache, conn}
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
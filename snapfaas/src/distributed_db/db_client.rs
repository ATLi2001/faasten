use std::net::TcpStream;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Arc, Mutex};
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

struct SyscallChannel {
    syscall: SC,
    send_chan: Option<Sender<bool>>,
}

#[derive(Debug, Clone)]
pub struct DbClient {
    // address: String,
    cache: r2d2::Pool<DbServerManager>,
    conn: r2d2::Pool<DbServerManager>,
    tx: Arc<Mutex<Sender<SyscallChannel>>>,
    rx: Arc<Mutex<Receiver<SyscallChannel>>>,
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
        // TODO - currently reading from global for testing purpose
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
        // special value of EXTERNALIZE is not put in db
        if value == "EXTERNALIZE".as_bytes() {
            let (ext_send, ext_recv) = channel();
            self.tx.lock().unwrap().send(
                SyscallChannel{syscall: sc, send_chan: Some(ext_send)}
            ).unwrap();
            // wait on response
            let _ = ext_recv.recv().unwrap();
        }
        else {
            let cache_conn = &mut self.cache.get().unwrap();
            let _ = send_sc_get_response(sc.clone(), cache_conn);
            self.tx.lock().unwrap().send(
                SyscallChannel{syscall: sc, send_chan: None}
            ).unwrap();
        }
    }

    fn add(&self, key: &[u8], value: &[u8]) -> bool {
        let sc = SC::WriteKey(syscalls::WriteKey {
            key: Vec::from(key), 
            value: Vec::from(value),
            flags: Some(WriteFlags::NO_OVERWRITE.bits()),
        });
        let cache_conn = &mut self.cache.get().unwrap();
        let resp = send_sc_get_response(sc.clone(), cache_conn);

        // need to be synchronous
        let (ext_send, ext_recv) = channel();
            self.tx.lock().unwrap().send(
                SyscallChannel{syscall: sc, send_chan: Some(ext_send)}
            ).unwrap();
        // wait on response
        let _ = ext_recv.recv().unwrap();

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
        let resp = send_sc_get_response(sc.clone(), cache_conn);

        // need to be synchronous
        let (ext_send, ext_recv) = channel();
        self.tx.lock().unwrap().send(
            SyscallChannel{syscall: sc, send_chan: Some(ext_send)}
        ).unwrap();
        // wait on response
        let _ = ext_recv.recv().unwrap();
        
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
        let (tx, rx) = channel();

        DbClient {
            cache, 
            conn, 
            tx: Arc::new(Mutex::new(tx)), 
            rx: Arc::new(Mutex::new(rx)), 
        }
    }

    pub fn start_dbclient(self) {
        let arc_self = Arc::new(self);
        let arc_self_clone = arc_self.clone();
        std::thread::spawn(move || {
            arc_self_clone.channel_listen();
        });
    }

    pub fn channel_listen(&self) {
        loop {
            let sc_chan = self.rx.lock().unwrap().recv().unwrap();
            let sc = sc_chan.syscall;
            if sc_chan.send_chan.is_some() {
                let ext_send = sc_chan.send_chan.unwrap();
                ext_send.send(true).unwrap();
            }

            let conn = &mut self.conn.get().unwrap();
            let _ = send_sc_get_response(sc, conn);
        }
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
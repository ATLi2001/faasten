use std::net::TcpStream;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Arc, Mutex};
use log::{debug, error};
use lmdb::WriteFlags;
use tikv_client::TransactionClient;

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

#[derive(Clone)]
pub struct DbClient {
    // address: String,
    cache: r2d2::Pool<DbServerManager>,
    // conn: r2d2::Pool<DbServerManager>,
    globaldb_client: TransactionClient, 
    tx: Arc<Mutex<Sender<SyscallChannel>>>,
    rx: Arc<Mutex<Receiver<SyscallChannel>>>,
}

// legacy for read key, write key, read dir, cas basic operations
impl DbService for DbClient {
    fn get(&self, key: Vec<u8>) -> Result<Vec<u8>, Error> {
        debug!("DbService get");
        let sc = SC::ReadKey(syscalls::ReadKey {key});
        let cache_conn = &mut self.cache.get().unwrap();
        send_sc_get_response(sc, cache_conn)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        debug!("DbService put");
        let key_clone = key.clone();
        let sc = SC::WriteKey(syscalls::WriteKey {key, value, flags: None});

        let cache_conn = &mut self.cache.get().unwrap();
        let resp = send_sc_get_response(sc.clone(), cache_conn);
        // special value of EXTERNALIZE is not put in db
        if key_clone == "EXTERNALIZE".as_bytes() {
            debug!("externalization happening");
            self.send_to_background_thread(sc, true);
        }
        else {
            self.send_to_background_thread(sc, false);
        }
        resp        
    }

    fn add(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::WriteKey(syscalls::WriteKey {key, value, flags: Some(WriteFlags::NO_OVERWRITE.bits())});
        let cache_conn = &mut self.cache.get().unwrap();
        let resp = send_sc_get_response(sc.clone(), cache_conn);
        self.send_to_background_thread(sc, true);
        resp
    }

    fn cas(&self, key: Vec<u8>, expected: Option<Vec<u8>>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::CompareAndSwap(syscalls::CompareAndSwap {key, expected, value});
        let cache_conn = &mut self.cache.get().unwrap();
        let resp = send_sc_get_response(sc.clone(), cache_conn);
        self.send_to_background_thread(sc, true);
        resp
    }
    
    fn scan(&self, dir: Vec<u8>) -> Result<Vec<u8>, Error> {
        let sc = SC::ReadDir(syscalls::ReadDir {dir});
        let conn = &mut self.cache.get().map_err(|_| Error::TcpConnectionError)?;
        send_sc_get_response(sc, conn)
    }
}

impl BackingStore for DbClient {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let sc = SC::ReadKey(syscalls::ReadKey {key: Vec::from(key)});
        let cache_conn = &mut self.cache.get().unwrap();
        let resp = send_sc_get_response(sc, cache_conn);
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
            self.send_to_background_thread(sc, true);
        }
        else {
            let cache_conn = &mut self.cache.get().unwrap();
            let _ = send_sc_get_response(sc.clone(), cache_conn);
            self.send_to_background_thread(sc, false);
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

        self.send_to_background_thread(sc, true);

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

        self.send_to_background_thread(sc, true);
        
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
    pub async fn new(address: String) -> Self {
        debug!("db_client created, server at {}", address.clone());
        let cache = r2d2::Pool::builder().max_size(10).build(DbServerManager { address: CACHE_ADDRESS.to_string() }).expect("cache pool");
        // let conn = r2d2::Pool::builder().max_size(10).build(DbServerManager { address: address.clone() }).expect("pool");
        let globaldb_client = TransactionClient::new(vec!["127.0.0.1:2379"], None).await.unwrap();
        let (tx, rx) = channel();

        DbClient {
            cache, 
            globaldb_client, 
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

    pub async fn channel_listen(&self) {
        let mut i = 0;
        loop {
            debug!("background thread count = {}", i);
            i += 1;
            let sc_chan = self.rx.lock().unwrap().recv().unwrap();
            let sc = sc_chan.syscall;

            match sc {
                SC::WriteKey(wk) => {
                    let mut flags = WriteFlags::empty();
                    if let Some(f) = wk.flags {
                        flags = WriteFlags::from_bits(f).expect("bad flags");
                    }

                    let mut txn = self.globaldb_client.begin_optimistic().await.unwrap();
                    if flags == WriteFlags::NO_OVERWRITE {
                        let key_exist = txn.key_exists(wk.key.to_owned()).await.unwrap();
                        if !key_exist {
                            txn.put(wk.key.to_owned(), wk.value.to_owned()).await.unwrap();
                        }
                    } 
                    else {
                        txn.put(wk.key.to_owned(), wk.value.to_owned()).await.unwrap();
                    }  
                    txn.commit().await.unwrap();
                },
                SC::CompareAndSwap(cas) => {
                    let mut txn = self.globaldb_client.begin_optimistic().await.unwrap();
                    let old = txn.get(cas.key.to_owned()).await.unwrap();
                    if cas.expected == old {
                        txn.put(cas.key.to_owned(), cas.value.to_owned()).await.unwrap();
                    }
                    txn.commit().await.unwrap();
                },
                _ => error!("unexpected syscall in db_client global_db_client {:?}", sc),
            };

            // let conn = &mut self.conn.get().unwrap();
            // let _ = send_sc_get_response(sc, conn);

            if sc_chan.send_chan.is_some() {
                let ext_send = sc_chan.send_chan.unwrap();
                ext_send.send(true).unwrap();
            }
        }
    }

    fn send_to_background_thread(&self, sc: SC, synchronous: bool) {
        if synchronous {
            debug!("send to background thread sync");
            let (ext_send, ext_recv) = channel();
            self.tx.lock().unwrap().send(
                SyscallChannel{syscall: sc, send_chan: Some(ext_send)}
            ).unwrap();
            // wait on response
            let _ = ext_recv.recv().unwrap();
        }
        else {
            debug!("send to background thread async");
            self.tx.lock().unwrap().send(
                SyscallChannel{syscall: sc, send_chan: None}
            ).unwrap();
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

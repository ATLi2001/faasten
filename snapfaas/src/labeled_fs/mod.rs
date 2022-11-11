use std::path::Path;

use log::{error, debug};
use rand::{self, RngCore};
use lazy_static;
use lmdb;
use lmdb::{Transaction, WriteFlags};
use labeled::dclabel::DCLabel;
use prost::Message;

mod dir;
mod file;
mod direntry;
pub mod utils;

use self::direntry::{LabeledDirEntry, DirEntry};
use self::dir::Directory;
use self::file::File;

use crate::syscalls;
use crate::distributed_db::DbService;

lazy_static::lazy_static! {
    pub static ref DBENV: lmdb::Environment = {
        if !std::path::Path::new("storage").exists() {
            let _ = std::fs::create_dir("storage").unwrap();
        }
        
        let dbenv = lmdb::Environment::new()
            .set_map_size(100 * 1024 * 1024 * 1024)
            .open(std::path::Path::new("storage"))
            .unwrap();

        // Create the root directory object at key 0 if not already exists.
        // `put_val_db_no_overwrite` uses `NO_OVERWRITE` as the write flag to make sure that it
        // will be a noop if the root already exists at key 0. And we can safely ignore the
        // returned `Result` here which if an error is an KeyExist error.
        let default_db = dbenv.open_db(None).unwrap();
        let mut txn = dbenv.begin_rw_txn().unwrap();
        let root_uid: u64 = 0;
        let _ = txn.put(default_db, &root_uid.to_be_bytes(), &Directory::new().to_vec(), WriteFlags::NO_OVERWRITE);
        txn.commit().unwrap();

        dbenv
    };
}

#[derive(PartialEq, Debug)]
pub enum Error {
    BadPath,
    Unauthorized,
    BadTargetLabel,
}

type Result<T> = std::result::Result<T, Error>;

//////////////
//   APIs   //
//////////////
/// read always succeeds by raising labels unless the target path is illegal
pub fn read(path: &str, cur_label: &mut DCLabel, db_client: &mut impl DbService) -> Result<Vec<u8>> {
    debug!("read {}", path);
    let res = get_direntry(path, cur_label, db_client).and_then(|labeled| -> Result<Vec<u8>> {
        let entry = labeled.unlabel(cur_label);
        match entry.entry_type() {
            DirEntry::F => {
                let file = get_val_db(entry.uid(), db_client).map(File::from_vec).unwrap();
                Ok(file.data())
            },
            DirEntry::D => Err(Error::BadPath),
        }
    });
    res
}

/// read always succeed by raising labels unless the target path is illegal
pub fn list(path: &str, cur_label: &mut DCLabel, db_client: &mut impl DbService) -> Result<Vec<String>> {
    let res = get_direntry(path, cur_label, db_client).and_then(|labeled| -> Result<Vec<String>> {
        let entry = labeled.unlabel(cur_label);
        match entry.entry_type() {
            DirEntry::D => {
                let dir = get_val_db(entry.uid(), db_client).map(Directory::from_vec).unwrap();
                Ok(dir.list())
            },
            DirEntry::F => Err(Error::BadPath),
        }
    });
    res
}

/// create_dir only fails when `cur_label` cannot flow to `label` or target directory's label
pub fn create_dir(base_dir: &str, name: &str, label: DCLabel, cur_label: &mut DCLabel, db_client: &mut impl DbService) -> Result<()> {
    debug!("create_dir at {}/{}", base_dir, name);
    create_common(base_dir, name, label, cur_label, Directory::new().to_vec(), DirEntry::D, db_client)
}

/// create_file only fails when `cur_label` cannot flow to `label` or target directory's label
pub fn create_file(base_dir: &str, name: &str, label: DCLabel, cur_label: &mut DCLabel, db_client: &mut impl DbService) -> Result<()> {
    debug!("create_file at {}/{}", base_dir, name);
    create_common(base_dir, name, label, cur_label, File::new().to_vec(), DirEntry::F, db_client)
}

/// write fails when `cur_label` cannot flow to the target file's label 
pub fn write(path: &str, data: Vec<u8>, cur_label: &mut DCLabel, db_client: &mut impl DbService) -> Result<()> { 
    debug!("write at {}", path);
    let res = get_direntry(path, cur_label,  db_client).and_then(|labeled| -> Result<()> {
        let entry = labeled.unlabel_write_check(cur_label)?;
        match entry.entry_type() {
            DirEntry::F => {
                let mut file = get_val_db(entry.uid(), db_client).map(File::from_vec).unwrap();
                file.write(data);
                let _ = put_val_db(entry.uid(), file.to_vec(), db_client);
                Ok(())
            }
            DirEntry::D => Err(Error::BadPath),
        }
    });
    res
}

/////////////
// helpers //
/////////////
// return a random u64
fn get_uid() -> u64 {
    let mut ret = rand::thread_rng().next_u64();
    // 0 is reserved by the system for the root directory
    if ret == 0 {
        ret = rand::thread_rng().next_u64();
    }
    ret
}

fn get_val_db(uid: u64, db_client: &mut impl DbService) -> std::result::Result<Vec<u8>, lmdb::Error> {
    let buf = db_client.get((&uid.to_be_bytes()).to_vec()).unwrap();
    let resp = syscalls::ReadKeyResponse::decode(buf.as_ref()).expect("read key response");
    return Ok(resp.value.unwrap());
}

fn put_val_db_no_overwrite(uid: u64, val: Vec<u8>, db_client: &mut impl DbService) -> std::result::Result<(), lmdb::Error> {
    let buf = db_client.add((&uid.to_be_bytes()).to_vec(), val).unwrap();
    let resp = syscalls::WriteKeyResponse::decode(buf.as_ref()).expect("write key resposne");
    if resp.success {
        return Ok(());
    }
    else {
        return Err(lmdb::Error::BadTxn); // temp fix; need a better way
    }
}

fn put_val_db(uid: u64, val: Vec<u8>, db_client: &mut impl DbService) -> std::result::Result<(), lmdb::Error> {
    let buf = db_client.put((&uid.to_be_bytes()).to_vec(), val).unwrap();
    let resp = syscalls::WriteKeyResponse::decode(buf.as_ref()).expect("write key resposne");
    if resp.success {
        return Ok(());
    }
    else {
        return Err(lmdb::Error::BadTxn); // temp fix; need a better way
    }
}

fn cas_db(uid: u64, expected: Option<Vec<u8>>, val: Vec<u8>, db_client: &mut impl DbService) -> std::result::Result<(), lmdb::Error> {
    let buf = db_client.cas((&uid.to_be_bytes()).to_vec(), expected, val).unwrap();
    let resp = syscalls::WriteKeyResponse::decode(buf.as_ref()).expect("write key resposne");
    if resp.success {
        return Ok(());
    }
    else {
        return Err(lmdb::Error::BadTxn); // temp fix; need a better way
    }
}

// return the labeled direntry named by the path
fn get_direntry(path: &str, cur_label: &mut DCLabel, db_client: &mut impl DbService) -> Result<LabeledDirEntry> {
    let path = Path::new(path);
    let mut labeled = LabeledDirEntry::root();
    let mut it = path.iter();
    let _ = it.next();
    for component in it {
        let entry = labeled.unlabel(cur_label);
        match entry.entry_type() {
            DirEntry::F => {
                return Err(Error::BadPath);
            },
            DirEntry::D => {
                let cur_dir = get_val_db(entry.uid(), db_client).map(Directory::from_vec).unwrap();
                labeled = cur_dir.get(component.to_str().unwrap())?.clone();
            },
        }
    }
    Ok(labeled)
}

fn create_common(
    base_dir: &str,
    name: &str,
    label: DCLabel,
    cur_label: &mut DCLabel,
    obj_vec: Vec<u8>,
    entry_type: DirEntry,
    db_client: &mut impl DbService,
) -> Result<()> {
    let res = get_direntry(base_dir, cur_label, db_client).and_then(|labeled| -> Result<()> {
        let entry = labeled.unlabel_write_check(cur_label)?;
        match entry.entry_type() {
            DirEntry::D => {
                let mut dir = get_val_db(entry.uid(), db_client).map(Directory::from_vec).unwrap();
                let old_dir_val = dir.to_vec();
                let mut uid = get_uid();
                while put_val_db_no_overwrite(uid, obj_vec.clone(), db_client).is_err() {
                    uid = get_uid();
                }
                dir.create(name, cur_label, entry_type, label, uid)?;
                let cas_res = cas_db(entry.uid(), Some(old_dir_val), dir.to_vec(), db_client);
                if cas_res.is_err() {
                    error!("cas failed");
                }
                Ok(())
            },
            DirEntry::F => Err(Error::BadPath),
        }
    });
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LocalDb {
        pub name: String,
    }

    impl DbService for LocalDb {
        fn get(&mut self, key: Vec<u8>) -> std::result::Result<Vec<u8>, crate::distributed_db::db_client::Error> {
            let db = DBENV.open_db(None).unwrap();
            let txn = DBENV.begin_ro_txn().unwrap();
            let result = syscalls::ReadKeyResponse {
                value: txn.get(db, &key).ok().map(Vec::from),
            }
            .encode_to_vec();
            let _ = txn.commit();
            Ok(result)
        }
        fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> std::result::Result<Vec<u8>, crate::distributed_db::db_client::Error> {
            let db = DBENV.open_db(None).unwrap();
            let mut txn = DBENV.begin_rw_txn().unwrap();

            let result = syscalls::WriteKeyResponse {
                success: txn
                    .put(db, &key, &value, WriteFlags::empty())
                    .is_ok(),
            }
            .encode_to_vec();
            let _ = txn.commit();
            Ok(result)
        }
        fn add(&mut self, key: Vec<u8>, value: Vec<u8>) -> std::result::Result<Vec<u8>, crate::distributed_db::db_client::Error> {
            let db = DBENV.open_db(None).unwrap();
            let mut txn = DBENV.begin_rw_txn().unwrap();

            let result = syscalls::WriteKeyResponse {
                success: txn
                    .put(db, &key, &value, WriteFlags::NO_OVERWRITE)
                    .is_ok(),
            }
            .encode_to_vec();
            let _ = txn.commit();
            Ok(result)
        }
        fn cas(&mut self, key: Vec<u8>, expected: Option<Vec<u8>>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
            let db = DBENV.open_db(None).unwrap();
            let mut txn = DBENV.begin_rw_txn().unwrap();
            let old = txn.get(db, &key).ok().map(Into::into);
            let res = if expected.map(|e| Vec::from(e)) == old {
                let _ = txn.put(db, &key, &value, WriteFlags::empty());
                Ok(())
            } else {
                Err(old)
            };
            txn.commit().unwrap();

            let result = syscalls::WriteKeyResponse {
                success: res.is_ok(),
            }
            .encode_to_vec();
            Ok(result)
        }
        fn scan(&mut self, dir: Vec<u8>) -> std::result::Result<Vec<u8>, crate::distributed_db::db_client::Error> {
            Ok(Vec::new())  // skipping for now
        }
    }

    #[test]
    fn test_storage_create_dir_list_fail() {
        let mut db_client = LocalDb {name: "testdb".to_string()};

        // create `/gh_repo`
        let target_label = DCLabel::new(true, [["gh_repo"]]);
        let mut cur_label = DCLabel::bottom();
        assert!(create_dir("/", "gh_repo", target_label, &mut cur_label, &mut db_client).is_ok());

        // list
        let mut cur_label = DCLabel::public();
        assert_eq!(list("/", &mut cur_label, &mut db_client).unwrap(), vec![String::from("gh_repo"); 1]);

        // already exists
        let target_label = DCLabel::new(true, [["gh_repo"]]);
        let mut cur_label = DCLabel::bottom();
        assert_eq!(create_dir("/", "gh_repo", target_label, &mut cur_label, &mut db_client).unwrap_err(), Error::BadPath);

        // missing path components
        let target_label = DCLabel::new([["yue"]], [["gh_repo"]]);
        let mut cur_label = DCLabel::public();
        assert_eq!(create_dir("/gh_repo/yue", "yue", target_label, &mut cur_label, &mut db_client).unwrap_err(), Error::BadPath);

        // label too high
        let target_label = DCLabel::new([["yue"]], [["gh_repo"]]);
        let mut cur_label = target_label.clone();
        assert_eq!(create_dir("/gh_repo", "yue", target_label, &mut cur_label, &mut db_client).unwrap_err(), Error::Unauthorized);

        // label too high
        let target_label = DCLabel::new([["yue"]], [["gh_repo"]]);
        let mut cur_label = DCLabel::new([["yue"]], true);
        assert_eq!(create_dir("/gh_repo", "yue", target_label, &mut cur_label, &mut db_client).unwrap_err(), Error::Unauthorized);

        // create /gh_repo/yue
        let target_label = DCLabel::new([["yue"]], [["gh_repo"]]);
        let mut cur_label = DCLabel::new(true, [["gh_repo"]]);
        assert!(create_dir("/gh_repo", "yue", target_label, &mut cur_label, &mut db_client).is_ok());

        // Unauthorized not BadPath
        let target_label = DCLabel::new([["yue"]], [["gh_repo"]]);
        let mut cur_label = DCLabel::public();
        assert_eq!(create_dir("/gh_repo", "yue", target_label, &mut cur_label, &mut db_client).unwrap_err(), Error::Unauthorized);
    }

    #[test]
    fn test_storage_create_file_write_read() {
        let mut db_client = LocalDb {name: "testdb".to_string()};

        // create `/func4`
        let mut cur_label = DCLabel::bottom();
        let target_label = DCLabel::new([["func4"]], [["func4"]]);
        assert!(create_dir("/", "func4", target_label, &mut cur_label, &mut db_client).is_ok());

        // create `/func4/mydata.txt`
        // after reading the directory /func4, cur_label gets raised to <func4, func4> and
        // cannot flow to the target label <user2, func4>
        let mut cur_label = DCLabel::new(true, [["func4"]]);
        let target_label = DCLabel::new([["user2"]], [["func4"]]);
        assert_eq!(create_file("/func4", "mydata.txt", target_label, &mut cur_label, &mut db_client).unwrap_err(), Error::BadTargetLabel);
        // <func4, func4> can flow to <user2/\func4, func4>
        let target_label = DCLabel::new([["user2"], ["func4"]], [["func4"]]);
        assert!(create_file("/func4", "mydata.txt", target_label, &mut cur_label, &mut db_client).is_ok());
        assert_eq!(read("/func4/mydata.txt", &mut cur_label, &mut db_client).unwrap(), Vec::<u8>::new());
    
        // write read
        let text = "test message";
        let data = text.as_bytes().to_vec();
        assert!(write("/func4/mydata.txt", data.clone(), &mut cur_label, &mut db_client).is_ok());
        assert_eq!(read("/func4/mydata.txt", &mut cur_label, &mut db_client).unwrap(), data);

        //// overwrite read
        let text = "test message test message";
        let data = text.as_bytes().to_vec();
        assert!(write("/func4/mydata.txt", data.clone(), &mut cur_label, &mut db_client).is_ok());
        assert_eq!(read("/func4/mydata.txt", &mut cur_label, &mut db_client).unwrap(), data);
    }
}

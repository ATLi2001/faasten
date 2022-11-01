use std::path::Path;

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
use crate::distributed_db::db_client::DbClient;

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
        let root_uid = 0;
        let _ = put_val_db_no_overwrite(root_uid, Directory::new().to_vec(), &mut txn, default_db);
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
pub fn read(path: &str, cur_label: &mut DCLabel, db_client: &mut DbClient) -> Result<Vec<u8>> {
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
pub fn list(path: &str, cur_label: &mut DCLabel, db_client: &mut DbClient) -> Result<Vec<String>> {
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
pub fn create_dir(base_dir: &str, name: &str, label: DCLabel, cur_label: &mut DCLabel, db_client: &mut DbClient) -> Result<()> {
    create_common(base_dir, name, label, cur_label, Directory::new().to_vec(), DirEntry::D, db_client)
}

/// create_file only fails when `cur_label` cannot flow to `label` or target directory's label
pub fn create_file(base_dir: &str, name: &str, label: DCLabel, cur_label: &mut DCLabel, db_client: &mut DbClient) -> Result<()> {
    create_common(base_dir, name, label, cur_label, File::new().to_vec(), DirEntry::F, db_client)
}

/// write fails when `cur_label` cannot flow to the target file's label 
pub fn write(path: &str, data: Vec<u8>, cur_label: &mut DCLabel, db_client: &mut DbClient) -> Result<()> { 
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

fn get_val_db(uid: u64, db_client: &mut DbClient) -> std::result::Result<Vec<u8>, lmdb::Error> {
    let buf = db_client.get((&uid.to_be_bytes()).to_vec()).unwrap();
    let resp = syscalls::ReadKeyResponse::decode(buf.as_ref()).expect("read key response");
    return Ok(resp.value.unwrap());
}

fn put_val_db_no_overwrite(uid: u64, val: Vec<u8>, txn: &mut lmdb::RwTransaction, db: lmdb::Database) -> std::result::Result<(), lmdb::Error> {
    txn.put(db, &uid.to_be_bytes(), &val, WriteFlags::NO_OVERWRITE)
}

fn put_val_db(uid: u64, val: Vec<u8>, db_client: &mut DbClient) -> std::result::Result<(), lmdb::Error> {
    let buf = db_client.put((&uid.to_be_bytes()).to_vec(), val).unwrap();
    let resp = syscalls::WriteKeyResponse::decode(buf.as_ref()).expect("write key resposne");
    if resp.success {
        return Ok(());
    }
    else {
        return Err(lmdb::Error::BadTxn); // temp fix; need a better way
    }
}

// return the labeled direntry named by the path
fn get_direntry(path: &str, cur_label: &mut DCLabel, db_client: &mut DbClient) -> Result<LabeledDirEntry> {
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
    db_client: &mut DbClient,
) -> Result<()> {
    let db = DBENV.open_db(None).unwrap();
    let mut txn = DBENV.begin_rw_txn().unwrap();
    let res = get_direntry(base_dir, cur_label, db_client).and_then(|labeled| -> Result<()> {
        let entry = labeled.unlabel_write_check(cur_label)?;
        match entry.entry_type() {
            DirEntry::D => {
                let mut dir = get_val_db(entry.uid(), db_client).map(Directory::from_vec).unwrap();
                let mut uid = get_uid();
                while put_val_db_no_overwrite(uid, obj_vec.clone(), &mut txn, db).is_err() {
                    uid = get_uid();
                }
                dir.create(name, cur_label, entry_type, label, uid)?;
                let _ = put_val_db(entry.uid(), dir.to_vec(), db_client);
                Ok(())
            },
            DirEntry::F => Err(Error::BadPath),
        }
    });
    txn.commit().unwrap();
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed_db::db_server::DbServer;

    #[test]
    fn test_storage_create_dir_list_fail() {
        let db_server = DbServer::new("127.0.0.1:7878".to_string());
        DbServer::start_dbserver(db_server);
        let mut db_client = DbClient::new("127.0.0.1:7878".to_string());

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
        let db_server = DbServer::new("127.0.0.1:7878".to_string());
        DbServer::start_dbserver(db_server);
        let mut db_client = DbClient::new("127.0.0.1:7878".to_string());

        // create `/func2`
        let mut cur_label = DCLabel::bottom();
        let target_label = DCLabel::new([["func2"]], [["func2"]]);
        assert!(create_dir("/", "func2", target_label, &mut cur_label, &mut db_client).is_ok());

        // create `/func2/mydata.txt`
        // after reading the directory /func2, cur_label gets raised to <func2, func2> and
        // cannot flow to the target label <user2, func2>
        let mut cur_label = DCLabel::new(true, [["func2"]]);
        let target_label = DCLabel::new([["user2"]], [["func2"]]);
        assert_eq!(create_file("/func2", "mydata.txt", target_label, &mut cur_label, &mut db_client).unwrap_err(), Error::BadTargetLabel);
        // <func2, func2> can flow to <user2/\func2, func2>
        let target_label = DCLabel::new([["user2"], ["func2"]], [["func2"]]);
        assert!(create_file("/func2", "mydata.txt", target_label, &mut cur_label, &mut db_client).is_ok());
        assert_eq!(read("/func2/mydata.txt", &mut cur_label, &mut db_client).unwrap(), Vec::<u8>::new());
    
        // write read
        let text = "test message";
        let data = text.as_bytes().to_vec();
        assert!(write("/func2/mydata.txt", data.clone(), &mut cur_label, &mut db_client).is_ok());
        assert_eq!(read("/func2/mydata.txt", &mut cur_label, &mut db_client).unwrap(), data);

        //// overwrite read
        let text = "test message test message";
        let data = text.as_bytes().to_vec();
        assert!(write("/func2/mydata.txt", data.clone(), &mut cur_label, &mut db_client).is_ok());
        assert_eq!(read("/func2/mydata.txt", &mut cur_label, &mut db_client).unwrap(), data);
    }
}

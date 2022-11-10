use labeled::dclabel::DCLabel;

use crate::labeled_fs;
use crate::distributed_db::db_client::DbClient;

const ROOT: &str = "/";

// return the bytes associated with a new directory
pub fn get_new_dir_bytes() -> Vec<u8>{
    labeled_fs::Directory::new().to_vec()
}

/// Utility function to create function directory under the root directory
pub fn create_root_function_dir(name: &str, db_client: &mut DbClient) {
    let mut cur_label = DCLabel::bottom();
    labeled_fs::create_dir(ROOT, name, DCLabel::new(true, [[name]]), &mut cur_label, db_client).unwrap();
}

/// Utility function to create user directory under the root directory
pub fn create_root_user_dir(user: &str, db_client: &mut DbClient) {
    let mut cur_label = DCLabel::bottom();
    labeled_fs::create_dir(ROOT, user, DCLabel::new([[user]], [[user]]), &mut cur_label, db_client).unwrap();
}

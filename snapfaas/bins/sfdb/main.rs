#[macro_use(crate_version, crate_authors)]
extern crate clap;
use clap::{App, Arg};

use snapfaas::distributed_db::{db_server::DbServer, db_client::DbClient, DbService, CACHE_ADDRESS};


fn main() {
    let cmd_arguments = App::new("fireruner wrapper")
        .version(crate_version!())
        .author(crate_authors!())
        .about("launch a single firerunner vm.")
        .arg(
            Arg::with_name("KEY")
                .required(true)
                .index(1)
            )
        .arg(
            Arg::with_name("VALUE")
                .required(false)
                .index(2)
        ).get_matches();

    let address = "127.0.0.1:7878".to_string();
    let near_db_server = DbServer::new("near_storage".to_string(), CACHE_ADDRESS.to_string());
    DbServer::start_dbserver(near_db_server);
    let far_db_server = DbServer::new("far_storage".to_string(), address.clone());
    DbServer::start_dbserver(far_db_server);
    std::thread::sleep(std::time::Duration::from_secs(1));

    let db_client = DbClient::new(address);
    let db_client_clone = db_client.clone();
    db_client_clone.start_dbclient();

    let key = cmd_arguments.value_of("KEY").unwrap();
    // put
    if let Some(value) = cmd_arguments.value_of("VALUE") {
        if value == "-" {
            let mut value_bytes = Vec::new();
            let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut value_bytes);

            let _ = db_client.put(Vec::from(key), value_bytes);
        }
        else {
            let _ = db_client.put(Vec::from(key), Vec::from(value));
        }
    }
    // get
    else {
        let output = db_client.get(Vec::from(key)).unwrap();
        println!("{}", String::from_utf8_lossy(&output));
    }

}

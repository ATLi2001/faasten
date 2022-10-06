#[macro_use(crate_version, crate_authors)]
extern crate clap;
use clap::{App, Arg, SubCommand};
use labeled::dclabel::DCLabel;
use std::io::{Read, Write};

use snapfaas::labeled_fs;
use snapfaas::cli_utils::*;

fn main() {
    let cmd_arguments = App::new("sffs")
        .version(crate_version!())
        .author(crate_authors!())
        .about("This program is a wrapper over the labeled_fs module. \
            The main goal is to serve as a tool to create and modify files in the file system. \
            The program outputs reads to any requested path to the stdin.")
        .subcommand(
            SubCommand::with_name("ls")
                .about("List the given directory")
                .arg(Arg::with_name("PATH")
                    .index(1)
                    .required(true)
                    .long_help(
                        "Slash separated paths whose components are either descriptive names (i.e., myphotos) or DC Labels
                        in the format of [SECRECY_CLAUSE1;SECRECY_CLAUSE2;...#INTEGRITY_CLAUSE1;INTEGRITY_CLAUSE2;...]
                        where each clause is a comma-separated string of principals."))
        )
        .subcommand(
            SubCommand::with_name("cat")
                .about("Ouput the given file to the stdout")
                .arg(Arg::with_name("PATH")
                    .index(1)
                    .required(true)
                    .long_help(
                        "Slash separated paths whose components are either descriptive names (i.e., myphotos) or DC Labels
                        in the format of [SECRECY_CLAUSE1;SECRECY_CLAUSE2;...#INTEGRITY_CLAUSE1;INTEGRITY_CLAUSE2;...]
                        where each clause is a comma-separated string of principals."))
        )
        .subcommand(
            SubCommand::with_name("mkdir")
                .about("Create a directory named by the given path with the given label")
                .arg(Arg::with_name("PATH")
                    .index(1)
                    .required(true)
                    .long_help(
                        "Slash separated paths whose components are either descriptive names (i.e., myphotos) or DC Labels
                        in the format of [SECRECY_CLAUSE1;SECRECY_CLAUSE2;...#INTEGRITY_CLAUSE1;INTEGRITY_CLAUSE2;...]
                        where each clause is a comma-separated string of principals."))
                .arg(Arg::with_name("secrecy")
                    .short("s")
                    .long("secrecy")
                    .multiple(true)
                    .value_delimiter(";")
                    .require_delimiter(true)
                    .value_name("SECRECY CLAUSE")
                    .required(true)
                    .help("A DCLabel clause is a string of comma-delimited principals. Multiple clauses must be delimited by semi-colons."))
                .arg(Arg::with_name("integrity")
                    .short("i")
                    .long("integrity")
                    .multiple(true)
                    .value_delimiter(";")
                    .require_delimiter(true)
                    .value_name("INTEGRITY CLAUSE")
                    .required(true)
                    .help("A DCLabel clause is a string of comma-delimited principals. Multiple clauses must be delimited by semi-colons."))
                .arg(Arg::with_name("endorse")
                    .short("e")
                    .long("endorse")
                    .required(true)
                    .takes_value(true)
                    .help("Endorse the creation with the given principal"))
                .arg(Arg::with_name("faceted")
                    .conflicts_with_all(&["secrecy", "integrity"])
                    .long("faceted")
                    .required(false)
                    .takes_value(false)
                    .help("If present, create a faceted directory labeled public with no overriding."))
        )
        .subcommand(
            SubCommand::with_name("mkfile")
                .about("Create a file named by the given path with the given label")
                .arg(Arg::with_name("PATH")
                    .index(1)
                    .required(true)
                    .long_help(
                        "Slash separated paths whose components are either descriptive names (i.e., myphotos) or DC Labels
                        in the format of [SECRECY_CLAUSE1;SECRECY_CLAUSE2;...#INTEGRITY_CLAUSE1;INTEGRITY_CLAUSE2;...]
                        where each clause is a comma-separated string of principals."))
                .arg(Arg::with_name("secrecy")
                    .short("s")
                    .long("secrecy")
                    .multiple(true)
                    .value_delimiter(";")
                    .require_delimiter(true)
                    .value_name("SECRECY CLAUSE")
                    .required(true)
                    .help("A DCLabel clause is a string of comma-delimited principals. Multiple clauses must be delimited by semi-colons."))
                .arg(Arg::with_name("integrity")
                    .short("i")
                    .long("integrity")
                    .multiple(true)
                    .value_delimiter(";")
                    .require_delimiter(true)
                    .value_name("INTEGRITY CLAUSE")
                    .required(true)
                    .help("A DCLabel clause is a string of comma-delimited principals. Multiple clauses must be delimited by semi-colons."))
                .arg(Arg::with_name("endorse")
                    .short("e")
                    .long("endorse")
                    .required(true)
                    .takes_value(true)
                    .help("Endorse the creation with the given principal"))
        )
        .subcommand(
            SubCommand::with_name("write")
                .about("Overwrite the given file with the data from the given file or the stdin")
                .arg(Arg::with_name("PATH")
                    .index(1)
                    .required(true)
                    .long_help(
                        "Slash separated paths whose components are either descriptive names (i.e., myphotos) or DC Labels
                        in the format of [SECRECY_CLAUSE1;SECRECY_CLAUSE2;...#INTEGRITY_CLAUSE1;INTEGRITY_CLAUSE2;...]
                        where each clause is a comma-separated string of principals."))
                .arg(Arg::with_name("FILE")
                    .short("f")
                    .long("file")
                    .takes_value(true)
                    .value_name("FILE"))
                .arg(Arg::with_name("endorse")
                    .short("e")
                    .long("endorse")
                    .required(true)
                    .takes_value(true)
                    .help("Endorse the modification with the given principal"))
        )
        .get_matches();

    let mut cur_label = DCLabel::public();
    match cmd_arguments.subcommand() {
        ("cat", Some(sub_m)) => {
            if let Ok(data) = labeled_fs::read(input_to_path(sub_m.value_of("PATH").unwrap()), &mut cur_label) {
                std::io::stdout().write_all(&data).unwrap();
            } else {
                eprintln!("Invalid path.");
            }
        },
        ("ls", Some(sub_m)) => {
            if let Ok(list) = labeled_fs::list(input_to_path(sub_m.value_of("PATH").unwrap()), &mut cur_label) {
                let output = list.join("\t");
                println!("{}", output);
            } else {
                eprintln!("Invalid path.");
            }
        },
        ("mkdir", Some(sub_m)) => {
            let parts: Vec<&str> = sub_m.value_of("PATH").unwrap().rsplitn(2, '/').collect();
            let base_dir = input_to_path(parts.get(1).expect("Malformed path"));
            let name = parts.get(0).expect("Malformed path");
            let s_clauses: Vec<&str> = sub_m.values_of("secrecy").unwrap().collect();
            let i_clauses: Vec<&str> = sub_m.values_of("integrity").unwrap().collect();
            cur_label = input_to_endorsement(sub_m.value_of("endorse").unwrap());
            if sub_m.is_present("faceted") {
                match labeled_fs::create_faceted_dir(base_dir, &name, &mut cur_label) {
                    Err(labeled_fs::Error::BadPath) => {
                        eprintln!("Invalid path.");
                    },
                    Err(labeled_fs::Error::Unauthorized) => {
                        eprintln!("Bad endorsement.");
                    },
                    Err(labeled_fs::Error::BadTargetLabel) => {
                        eprintln!("Bad target label.");
                    },
                    Ok(()) => {},
                }
            } else {
                match labeled_fs::create_dir(
                    base_dir,
                    &name,
                    input_to_dclabel([s_clauses, i_clauses]),
                    &mut cur_label) {
                    Err(labeled_fs::Error::BadPath) => {
                        eprintln!("Invalid path.");
                    },
                    Err(labeled_fs::Error::Unauthorized) => {
                        eprintln!("Bad endorsement.");
                    },
                    Err(labeled_fs::Error::BadTargetLabel) => {
                        eprintln!("Bad target label.");
                    },
                    Ok(()) => {},
                }
            }
        },
        ("mkfile", Some(sub_m)) => {
            let parts: Vec<&str> = sub_m.value_of("PATH").unwrap().rsplitn(2, '/').collect();
            let base_dir = input_to_path(parts.get(1).expect("Malformed path"));
            let name = parts.get(0).expect("Malformed path");
            let s_clauses: Vec<&str> = sub_m.values_of("secrecy").unwrap().collect();
            let i_clauses: Vec<&str> = sub_m.values_of("integrity").unwrap().collect();
            cur_label = input_to_endorsement(sub_m.value_of("endorse").unwrap());
            match labeled_fs::create_file(
                base_dir,
                &name,
                input_to_dclabel([s_clauses, i_clauses]),
                &mut cur_label) {
                Err(labeled_fs::Error::BadPath) => {
                    eprintln!("Invalid path.");
                },
                Err(labeled_fs::Error::Unauthorized) => {
                    eprintln!("Bad endorsement.");
                },
                Err(labeled_fs::Error::BadTargetLabel) => {
                    eprintln!("Bad target label.");
                },
                Ok(()) => {},
            }
        },
        ("write", Some(sub_m)) => {
            let data = sub_m.value_of("FILE").map_or_else(
                || {
                    let mut buf = Vec::new();
                    std::io::stdin().read_to_end(&mut buf).unwrap();
                    buf
                },
                |p| std::fs::read(p).unwrap()
            );
            cur_label = input_to_endorsement(sub_m.value_of("endorse").unwrap());
            match labeled_fs::write(input_to_path(sub_m.value_of("PATH").unwrap()), data, &mut cur_label) {
                Err(labeled_fs::Error::BadPath) => {
                    eprintln!("Invalid path.");
                },
                Err(labeled_fs::Error::Unauthorized) => {
                    eprintln!("Bad endorsement.");
                },
                Err(labeled_fs::Error::BadTargetLabel) => {
                    eprintln!("write should not reach here.");
                },
                Ok(()) => {},
            }
        },
        (&_, _) => {
            eprintln!("{}", cmd_arguments.usage());
        }
    }
}

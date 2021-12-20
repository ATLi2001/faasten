use clap::{App, Arg};

mod app;
mod config;
mod server;

fn main() -> Result<(), std::io::Error> {
    env_logger::init();

    let matches = App::new("webhook server")
        .arg(
            Arg::with_name("snapfaas address")
                .short("p")
                .long("snapfaas_address")
                .value_name("[ADDR:]PORT")
                .takes_value(true)
                .required(true)
                .help("Path to snapfaas config YAML file"),
        )
        .arg(
            Arg::with_name("app config")
                .short("a")
                .long("app_config")
                .value_name("YAML")
                .takes_value(true)
                .required(true)
                .help("Path to app config YAML file"),
        )
        .arg(
            Arg::with_name("listen")
                .long("listen")
                .short("l")
                .takes_value(true)
                .value_name("ADDR:PORT")
                .required(true)
                .help("Address to listen on"),
        )
        .get_matches();

    let app = app::App::new(matches.value_of("app config").unwrap());
    let server = server::Server::new(
        matches.value_of("snapfaas address").unwrap().to_string(),
        matches.value_of("listen").unwrap(),
        app
    );
    server.run()
}

#[macro_use]
extern crate clap;
extern crate mush;
extern crate simplelog;

use clap::{App, Arg};
use simplelog::{TermLogger, LevelFilter, Config};
use mush::config;

fn main() {
    let matches = App::new("zris")
        .version("0.0.1")
        .author("Zeyi Fan <i@zr.is>")
        .about("Behind zr.is")
        .arg(Arg::with_name("host")
             .short("h")
             .long("host")
             .value_name("HOST")
             .default_value("127.0.0.1"))
        .arg(Arg::with_name("port")
             .short("p")
             .long("port")
             .value_name("PORT")
             .default_value("4000"))
        .arg(Arg::with_name("thread")
             .short("t")
             .long("thread")
             .value_name("NUM")
             .default_value("8"))
        .arg(Arg::with_name("database")
             .long("database")
             .value_name("URL")
             .required(true))
        .arg(Arg::with_name("debug")
             .short("d")
             .long("debug"))
        .arg(Arg::with_name("secret")
             .short("s")
             .long("secret"))
        .get_matches();

    let host = matches.value_of("host").unwrap();
    let port = value_t!(matches, "port", u16).unwrap();
    let thread = value_t!(matches, "thread", usize).unwrap();

    if matches.is_present("debug") {
        let _ = TermLogger::init(LevelFilter::Info, Config::default());
    } else {
        let _ = TermLogger::init(LevelFilter::Error, Config::default());
    }

    let database_url = matches.value_of("database").unwrap();

    let config = config::Config::new(
        matches.value_of("secret").map(str::to_string),
        database_url.to_string(),
    );

    mush::run(host, port, thread, config);
}
use clap::Parser;
use hakase::config;
use simplelog::{Config, LevelFilter, TermLogger};

#[derive(Parser, Debug)]
#[command(name = "Hakase")]
#[command(version = "0.0.1")]
#[command(author = "Zeyi Fan <i@zr.is>")]
#[command(about = "A URL shorter.", long_about = None)]
struct Args {
    /// Host to listen on
    #[arg(short, long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 4000)]
    port: u16,

    /// Number of threads
    #[arg(short, long, default_value_t = 8)]
    thread: usize,

    /// Database connection URL
    #[arg(long, required = true)]
    database: String,

    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,

    /// Secret key for signing
    #[arg(short, long)]
    secret: Option<String>,
}

fn main() {
    let args = Args::parse();

    if args.debug {
        let _ = TermLogger::init(LevelFilter::Debug, Config::default());
    } else {
        let _ = TermLogger::init(LevelFilter::Error, Config::default());
    }

    let host = &args.host;
    let port = args.port;
    let thread = args.thread;
    let database_url = args.database.clone();
    let config = config::Config::new(args.secret.clone(), database_url);

    hakase::run(host, port, thread, config);
}

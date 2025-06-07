use clap::Parser;
use hakase::config;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "Hakase")]
#[command(version = "0.0.1")]
#[command(author = "Zeyi Fan <i@zr.is>")]
#[command(about = "A URL shorter.", long_about = None)]
struct Args {
    /// Host to listen on
    #[arg(long, default_value = "127.0.0.1")]
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let filter = if args.debug {
        EnvFilter::from_default_env().add_directive("hakase=debug".parse().unwrap())
    } else {
        EnvFilter::from_default_env().add_directive("hakase=info".parse().unwrap())
    };

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    let host = &args.host;
    let port = args.port;
    let database_url = args.database.clone();
    let config = config::Config::new(args.secret.clone(), database_url);

    hakase::run(host, port, config).await;
}

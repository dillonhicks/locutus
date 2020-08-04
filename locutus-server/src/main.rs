pub(crate) mod deps {
    pub use bincode;
    pub use crossbeam;
    pub use futures_util;
    pub use locutus_actor;
    pub use locutus_game_of_life as gameoflife;
    pub use parking_lot;
    pub use rand;
    pub use rayon;
    pub use serde;
    pub use serde_json;
    pub use structopt;
    pub use tokio;
    pub use tokio_tungstenite;
    pub use tracing;
    pub use tracing_subscriber;
    pub use tungstenite;
}

mod actors;
mod cli;
mod logger;
mod server;

#[tokio::main]
async fn main() {
    use crate::deps::structopt::StructOpt;
    let args = crate::cli::Args::from_args();
    crate::logger::try_initialize(Some(args.log_level)).expect("could not initialize logger");

    let mut config = crate::server::Config::default();
    config.ip = args.host;
    config.port = args.port;
    config.tick = std::time::Duration::from_millis(1000 / args.tick_hertz);
    config.sim_threads = args.sim_threads;

    crate::server::serve(config).await.expect("failed to run server");
}

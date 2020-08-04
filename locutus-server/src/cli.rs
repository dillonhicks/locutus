use crate::deps::{
    structopt::StructOpt,
    tracing::Level,
};

use std::net::IpAddr;

#[derive(Debug, StructOpt)]
#[structopt(name = "locutus-server", about = "simulation server")]
pub struct Args {
    #[structopt(short, long, default_value = "info")]
    pub log_level: Level,

    #[structopt(short, long, default_value = "127.0.0.1")]
    pub host: IpAddr,

    #[structopt(short, long, default_value = "9001")]
    pub port: u16,

    #[structopt(short, long, default_value = "30")]
    pub tick_hertz: u64,

    #[structopt(long, default_value = "16")]
    pub sim_threads: usize,
}

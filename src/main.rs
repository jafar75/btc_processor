mod data_types;
mod processor;
mod data_generator;

use tracing::{info};
use tracing::metadata::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use crate::data_generator::{initialize_daemon, initialize_wallets};
use crate::processor::main_loop;


fn main() {
    // first initialize the tracing crate
    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(tracing_subscriber::fmt::Layer::new())
        .init();

    info!("Starting up");
    
    let bitcoind = initialize_daemon();
    let wallets = initialize_wallets(10, 100, &bitcoind);
    info!("wallets: {:?}", wallets);
    
    main_loop(wallets, bitcoind);
}

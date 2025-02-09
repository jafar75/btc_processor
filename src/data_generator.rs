use bitcoind::bitcoincore_rpc::bitcoin::Amount;
use bitcoind::bitcoincore_rpc::RpcApi;
use bitcoind::BitcoinD;
use rand::prelude::ThreadRng;
use rand::Rng;
use rand::seq::index::sample;
use tracing::info;
use uuid::Uuid;
use crate::data_types::{RandomTx, Wallet};

pub fn initialize_daemon() -> BitcoinD {
    let mut conf = bitcoind::Conf::default();
    conf.args = vec!["-regtest", "-fallbackfee=0.0001", "-txindex=1"];

    BitcoinD::from_downloaded_with_conf(&conf).unwrap()
}

pub fn generate_random_simulated_transaction(wallets: &[Wallet], rng: &mut ThreadRng) -> RandomTx {
    let indices = sample(rng, wallets.len(), 2);
    let amount: u64 = rng.random_range(1..1_000_000_000);
    RandomTx {
        unique_id: Uuid::new_v4(),
        sender: wallets[indices.index(0)].address.clone(),
        receiver: wallets[indices.index(1)].address.clone(),
        amount: Amount::from_sat(amount),
    }
}

fn generate_wallets(num_wallets: u32, daemon: &BitcoinD) -> Vec<Wallet>
{
    let mut wallets: Vec<Wallet> = Vec::with_capacity(num_wallets as usize);

    for i in 0..num_wallets {
        let name = format!("wallet_{}", i);
        let client = daemon.create_wallet(&name).unwrap();
        let address = client.get_new_address(None, None).unwrap().assume_checked();

        wallets.push(Wallet {client, address});
    }

    wallets
}

pub fn initialize_wallets(num_wallets: u32, balance_in_btc: u32, daemon: &BitcoinD) -> Vec<Wallet>
{
    let cl = &daemon.client;
    let estimated_needed_blocks = 100 + (balance_in_btc / 50u32) + 1u32;
    
    let mut wallets = generate_wallets(num_wallets, daemon);
    
    for wallet in wallets.iter_mut() {
        cl.generate_to_address(estimated_needed_blocks as u64, &wallet.address).unwrap();
    }

    for wallet in &wallets {
        let wallet_balance = wallet.client.get_balances().unwrap();
        info!("wallet_balance: {:?}", wallet_balance);
    }
    wallets
}
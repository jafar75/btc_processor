use bitcoind::bitcoincore_rpc::bitcoin::{Address, Amount};
use bitcoind::bitcoincore_rpc::Client;
use uuid::Uuid;

#[derive(Debug)]
pub struct Wallet {
    pub client: Client,
    pub address: Address,
}

#[derive(Debug)]
pub struct RandomTx {
    pub unique_id: Uuid,
    pub sender: Address,
    pub receiver: Address,
    pub amount: Amount,
}
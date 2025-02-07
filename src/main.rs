use std::{thread, time::Duration};

use bitcoind::bitcoincore_rpc::{bitcoin::{Amount, BlockHash}, Client, RpcApi};


fn mine(cl: &Client, n: usize) {
    let blocks = cl
        .generate_to_address(
            n as u64,
            &cl.get_new_address(None, None).unwrap().assume_checked(),
        )
        .unwrap();
    assert_eq!(blocks.len(), n);
}



fn main() {

    use bitcoind::bitcoincore_rpc::RpcApi as _;
    let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();
    let cl = &bitcoind.client;
    assert_eq!(0, cl.get_blockchain_info().unwrap().blocks);

    mine(cl, 101);
    assert_eq!(
        cl.get_balance(Some(1), None).unwrap(),
        Amount::from_int_btc(50)
    );

    let aa = cl.get_blockchain_info().unwrap().best_block_hash;
    let bb = cl.get_block(&aa).unwrap();

    loop {
        let amt = 2;
        let send_to = cl.get_new_address(None, None).unwrap();
        let txid = cl.send_to_address(&send_to.clone().assume_checked(), Amount::from_int_btc(amt), None, None, None, None, None, None).unwrap();
        
        thread::sleep(Duration::from_secs(1));
        println!("send amout: {:?} to address: {:?} ", amt, send_to);
    }

    // cl.create_raw_transaction(utxos, outs, locktime, replaceable)
}

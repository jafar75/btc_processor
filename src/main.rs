use std::{thread, time::Duration};

use bitcoind::bitcoincore_rpc::{bitcoin::{Amount, BlockHash}, json, Client, RpcApi};
use bitcoind::bitcoincore_rpc::json::AddressType::{Bech32, Legacy};

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
    let mut conf = bitcoind::Conf::default();
    conf.args = vec!["-regtest", "-fallbackfee=0.0001", "-txindex=1"];

    let bitcoind = bitcoind::BitcoinD::from_downloaded_with_conf(&conf).unwrap();

    let alice = bitcoind.create_wallet("alice").unwrap();
    let bob = bitcoind.create_wallet("bob").unwrap();

    let alice_address = alice.get_new_address(Some("alice_wallet"), None).unwrap().assume_checked();
    let bob_address = bob.get_new_address(Some("bob_wallet"), None).unwrap().assume_checked();

    println!("Alice address: {}", alice_address);
    println!("Bob address: {}", bob_address);

    // let alice_address_info = alice.get_address_info(&alice_address);
    // println!("Alice address info: {:?}", alice_address_info);
    let cl = &bitcoind.client;
    cl.generate_to_address(1, &alice_address).unwrap();
    cl.generate_to_address(101, &bob_address).unwrap();

    let balances = alice.get_balances().unwrap();
    let alice_balances: json::GetBalancesResult = balances;

    let balances = bob.get_balances().unwrap();
    let bob_balances: json::GetBalancesResult = balances;

    assert_eq!(
        Amount::from_btc(50.0).unwrap(),
        alice_balances.mine.trusted
    );
    assert_eq!(
        Amount::from_btc(50.0).unwrap(),
        bob_balances.mine.trusted
    );
    assert_eq!(
        Amount::from_btc(5000.0).unwrap(),
        bob_balances.mine.immature
    );

    let txid = alice.send_to_address(&bob_address, Amount::from_btc(1.0).unwrap(), Some("from alice to bob"), None, None, None, None, None).unwrap();
    // let tx = alice.get_transaction(&txid, None).unwrap();
    // println!("tx   {:?}\n\n\n\n", tx);

    // // let tt = cl.decode_raw_transaction(&tx.hex, None).expect("TODO: panic message");
    // // let vin_tx = tt.vin.first().unwrap().txid.unwrap();
    // let ttt = cl.get_raw_transaction_info(&txid, None).unwrap();
    // println!("ttt    {:?}\n\n\n", ttt);
    // // println!("tt    {:?}\n\n\n", tt);
    //
    // let vv = ttt.vin.first().unwrap().txid.unwrap();
    // let v = cl.decode_raw_transaction(&ttt.hex, None).unwrap();
    // println!("v     {:?}\n\n\n", v);


    //////
    let r = cl.get_raw_transaction_hex(&txid, None).unwrap();
    let d = cl.decode_raw_transaction(r, None).unwrap();
    println!("d     {:?}\n\n\n", d);

    let i = d.vin.last().unwrap().txid.unwrap();
    let r_i = cl.get_raw_transaction_hex(&i, None).unwrap();
    let d_i = cl.decode_raw_transaction(r_i, None).unwrap();
    println!("d_i   {:?}\n\n\n", d_i);




    let hashes = cl.generate_to_address(1, &alice_address).unwrap();
    let latest_block = cl.get_block(hashes.last().unwrap());
    // println!("{:#?}", latest_block);

    let balances = alice.get_balances().unwrap();
    let alice_balances: json::GetBalancesResult = balances;

    assert!(
        alice_balances.mine.trusted
            < Amount::from_btc(49.0).unwrap()
            && alice_balances.mine.trusted
            > Amount::from_btc(48.9).unwrap()
    );

    // bob wallet may not be immediately updated
    for _ in 0..30 {
        let balances = bob.get_balances().unwrap();
        let bob_balances: json::GetBalancesResult = balances;

        if bob_balances.mine.untrusted_pending.to_sat() > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let balances = bob.get_balances().unwrap();
    let bob_balances: json::GetBalancesResult = balances;

    assert_eq!(
        Amount::from_btc(1.0).unwrap(),
        bob_balances.mine.untrusted_pending
    );
    assert!(bitcoind.create_wallet("bob").is_err(), "wallet already exist");




    // let cl = &bitcoind.client;
    // assert_eq!(0, cl.get_blockchain_info().unwrap().blocks);
    //
    // mine(cl, 101);
    // assert_eq!(
    //     cl.get_balance(Some(1), None).unwrap(),
    //     Amount::from_int_btc(50)
    // );
    //
    // // create some wallets
    // let w1_res = cl.create_wallet("w1", None, None, None, None)
    //     .expect("TODO: panic w1");
    //
    //
    // cl.create_wallet("w2", None, None, None, None)
    //     .expect("TODO: panic w2");
    //
    // let wallets = cl.list_wallets().expect("TODO: panic message");
    // println!("{:?}", wallets);
    //
    // let aa = cl.get_blockchain_info().unwrap().best_block_hash;
    // let bb = cl.get_block(&aa).unwrap();
    // loop {
    //     let amt = 2;
    //     let send_to = cl.get_new_address(None, None).unwrap();
    //     let txid = cl.send_to_address(&send_to.clone().assume_checked(), Amount::from_int_btc(amt), None, None, None, None, None, None).unwrap();
    //     thread::sleep(Duration::from_secs(1));
    //     println!("send amout: {:?} to address: {:?} ", amt, send_to);
    // }

    // cl.create_raw_transaction(utxos, outs, locktime, replaceable)
}

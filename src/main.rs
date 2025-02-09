use std::{thread, time::Duration};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::mpsc;
use bitcoind::bitcoincore_rpc::{bitcoin::{Amount, BlockHash}, json, Client, RpcApi};
use bitcoind::bitcoincore_rpc::bitcoin::{Address, Txid};
use bitcoind::BitcoinD;
use crossbeam_channel::unbounded;
use rand::{rng, thread_rng, Rng};
use rand::rngs::ThreadRng;
use rand::seq::index::sample;
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

fn mine(cl: &Client, n: usize) {
    let blocks = cl
        .generate_to_address(
            n as u64,
            &cl.get_new_address(None, None).unwrap().assume_checked(),
        )
        .unwrap();
    assert_eq!(blocks.len(), n);
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

fn initialize_wallets(wallets: &mut Vec<Wallet>, balance_in_btc: u32, daemon: &BitcoinD)
{
    // TODO generate address using cl, then mine and transfer to each of wallets equal values
    let cl = &daemon.client;
    let estimated_needed_blocks = 100 + (balance_in_btc / 50u32) + 1u32;

    for wallet in wallets.iter_mut() {
        cl.generate_to_address(estimated_needed_blocks as u64, &wallet.address).unwrap();
    }

    for wallet in wallets {
        let wallet_balance = wallet.client.get_balances().unwrap();
        println!("wallet_balance: {:?}", wallet_balance);
    }
}

fn generate_random_transaction(wallets: &Vec<Wallet>, rng: &mut ThreadRng) -> Txid
{
    let mut indices = sample(rng, wallets.len(), 2);
    let tx_id = wallets[indices.index(0)].client.send_to_address(
        &wallets[indices.index(1)].address, Amount::from_btc(1000.0).unwrap(), None,
        None, None, None, None, None
    ).unwrap();
    tx_id
}

fn generate_random_simulated_transaction(wallets: &Vec<Wallet>, rng: &mut ThreadRng) -> RandomTx {
    let indices = sample(rng, wallets.len(), 2);
    let amount: u64 = rng.random_range(1..1_000_000_000);
    println!("amount: {:?}", amount);
    RandomTx {
        unique_id: Uuid::new_v4(),
        sender: wallets[indices.index(0)].address.clone(),
        receiver: wallets[indices.index(1)].address.clone(),
        amount: Amount::from_sat(amount),
    }
}

fn main() {

    use bitcoind::bitcoincore_rpc::RpcApi as _;
    let mut conf = bitcoind::Conf::default();
    conf.args = vec!["-regtest", "-fallbackfee=0.0001", "-txindex=1"];

    let bitcoind = BitcoinD::from_downloaded_with_conf(&conf).unwrap();
    let cl = &bitcoind.client;

    let mut wallets = generate_wallets(10, &bitcoind);
    println!("wallets: {:?}", wallets);

    initialize_wallets(&mut wallets, 100, &bitcoind);

    let (sender, receiver) = unbounded();

    let mut map: HashMap<Address, Amount> = HashMap::new();

    thread::scope(
        |scope| {
            scope.spawn( || {
                let mut rng = rng();
                let mut cnt = 0;
                loop {
                    let tx_id = generate_random_simulated_transaction(&wallets, &mut rng);
                    sender.send(tx_id).unwrap();
                    thread::sleep(Duration::from_secs(1));
                    if cnt > 10 {
                        drop(sender);
                        break;
                    }
                    cnt += 1;
                }
            });

            scope.spawn( || {
                loop {
                    let x = receiver.recv_timeout(Duration::from_secs(5));
                    match x {
                        Ok(random_tx) => {
                            println!("random_tx: {:?}", random_tx);
                            // now using addresses of sender and receiver, find the wallet and
                            // then using rpc calls, check the balance, positivity of amount, etc
                            // also check the uuid to prevent double spending
                            let sender_address = &random_tx.sender;
                            let amount = &random_tx.amount;
                            if amount.le(&Amount::ZERO) {
                                continue;
                            }
                            for wallet in &wallets {
                                if wallet.address.eq(sender_address) {
                                    let balance = wallet.client.get_balances().unwrap();
                                    println!("balance: {:?}", balance);
                                    if balance.mine.trusted.lt(amount) {
                                        println!("insufficient balance!");
                                    } else {
                                        if map.contains_key(sender_address) {
                                            let current = map.get_mut(sender_address).unwrap();
                                            *current -= *amount;
                                        } else {
                                            map.insert(sender_address.clone(), balance.mine.trusted - *amount);
                                        }
                                    }
                                    break;
                                }
                            }

                            // let tx_id = Txid::from_str(&random_tx).unwrap();
                            // let r = cl.get_raw_transaction_hex(&tx_id, None).unwrap();
                            // let d = cl.decode_raw_transaction(r, None).unwrap();
                            // println!("d     {:?}\n\n\n", d);
                            //
                            // let i = d.vin.last().unwrap().txid.unwrap();
                            // let r_i = cl.get_raw_transaction_hex(&i, None).unwrap();
                            // let d_i = cl.decode_raw_transaction(r_i, None).unwrap();
                            // println!("d_i   {:?}\n\n\n", d_i);
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            });
        }
    );

    println!("wallet balances: {:?}", map);

//////////////////////////////////////////////////////////////////////////////////
//     let alice = bitcoind.create_wallet("alice").unwrap();
//     let bob = bitcoind.create_wallet("bob").unwrap();
//
//     let alice_address = alice.get_new_address(None, None).unwrap().assume_checked();
//     let bob_address = bob.get_new_address(None, None).unwrap().assume_checked();
//
//     println!("Alice address: {}", alice_address);
//     println!("Bob address: {}", bob_address);
//
//     // let alice_address_info = alice.get_address_info(&alice_address);
//     // println!("Alice address info: {:?}", alice_address_info);
//     let cl = &bitcoind.client;
//     cl.generate_to_address(1, &alice_address).unwrap();
//     cl.generate_to_address(101, &bob_address).unwrap();
//
//     let balances = alice.get_balances().unwrap();
//     let alice_balances: json::GetBalancesResult = balances;
//
//     let balances = bob.get_balances().unwrap();
//     let bob_balances: json::GetBalancesResult = balances;
//
//     assert_eq!(
//         Amount::from_btc(50.0).unwrap(),
//         alice_balances.mine.trusted
//     );
//     assert_eq!(
//         Amount::from_btc(50.0).unwrap(),
//         bob_balances.mine.trusted
//     );
//     assert_eq!(
//         Amount::from_btc(5000.0).unwrap(),
//         bob_balances.mine.immature
//     );
//
//     let txid = alice.send_to_address(&bob_address, Amount::from_btc(1.0).unwrap(), Some("from alice to bob"), None, None, None, None, None).unwrap();
//     // let tx = alice.get_transaction(&txid, None).unwrap();
//     // println!("tx   {:?}\n\n\n\n", tx);
//
//     // // let tt = cl.decode_raw_transaction(&tx.hex, None).expect("TODO: panic message");
//     // // let vin_tx = tt.vin.first().unwrap().txid.unwrap();
//     // let ttt = cl.get_raw_transaction_info(&txid, None).unwrap();
//     // println!("ttt    {:?}\n\n\n", ttt);
//     // // println!("tt    {:?}\n\n\n", tt);
//     //
//     // let vv = ttt.vin.first().unwrap().txid.unwrap();
//     // let v = cl.decode_raw_transaction(&ttt.hex, None).unwrap();
//     // println!("v     {:?}\n\n\n", v);
//
//
//     //////
//     let r = cl.get_raw_transaction_hex(&txid, None).unwrap();
//     let d = cl.decode_raw_transaction(r, None).unwrap();
//     println!("d     {:?}\n\n\n", d);
//
//     let i = d.vin.last().unwrap().txid.unwrap();
//     let r_i = cl.get_raw_transaction_hex(&i, None).unwrap();
//     let d_i = cl.decode_raw_transaction(r_i, None).unwrap();
//     println!("d_i   {:?}\n\n\n", d_i);
//
//
//
//
//     let hashes = cl.generate_to_address(1, &alice_address).unwrap();
//     let latest_block = cl.get_block(hashes.last().unwrap());
//     // println!("{:#?}", latest_block);
//
//     let balances = alice.get_balances().unwrap();
//     let alice_balances: json::GetBalancesResult = balances;
//
//     assert!(
//         alice_balances.mine.trusted
//             < Amount::from_btc(49.0).unwrap()
//             && alice_balances.mine.trusted
//             > Amount::from_btc(48.9).unwrap()
//     );
//
//     // bob wallet may not be immediately updated
//     for _ in 0..30 {
//         let balances = bob.get_balances().unwrap();
//         let bob_balances: json::GetBalancesResult = balances;
//
//         if bob_balances.mine.untrusted_pending.to_sat() > 0 {
//             break;
//         }
//         std::thread::sleep(std::time::Duration::from_millis(100));
//     }
//     let balances = bob.get_balances().unwrap();
//     let bob_balances: json::GetBalancesResult = balances;
//
//     assert_eq!(
//         Amount::from_btc(1.0).unwrap(),
//         bob_balances.mine.untrusted_pending
//     );
//     assert!(bitcoind.create_wallet("bob").is_err(), "wallet already exist");
//
//
//
//
//     // let cl = &bitcoind.client;
//     // assert_eq!(0, cl.get_blockchain_info().unwrap().blocks);
//     //
//     // mine(cl, 101);
//     // assert_eq!(
//     //     cl.get_balance(Some(1), None).unwrap(),
//     //     Amount::from_int_btc(50)
//     // );
//     //
//     // // create some wallets
//     // let w1_res = cl.create_wallet("w1", None, None, None, None)
//     //     .expect("TODO: panic w1");
//     //
//     //
//     // cl.create_wallet("w2", None, None, None, None)
//     //     .expect("TODO: panic w2");
//     //
//     // let wallets = cl.list_wallets().expect("TODO: panic message");
//     // println!("{:?}", wallets);
//     //
//     // let aa = cl.get_blockchain_info().unwrap().best_block_hash;
//     // let bb = cl.get_block(&aa).unwrap();
//     // loop {
//     //     let amt = 2;
//     //     let send_to = cl.get_new_address(None, None).unwrap();
//     //     let txid = cl.send_to_address(&send_to.clone().assume_checked(), Amount::from_int_btc(amt), None, None, None, None, None, None).unwrap();
//     //     thread::sleep(Duration::from_secs(1));
//     //     println!("send amout: {:?} to address: {:?} ", amt, send_to);
//     // }
//
//     // cl.create_raw_transaction(utxos, outs, locktime, replaceable)
}

use std::collections::HashSet;
use std::{env, thread};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU16, AtomicU8};
use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, Instant};
use bitcoind::bitcoincore_rpc::bitcoin::{Address, Amount};
use bitcoind::bitcoincore_rpc::RpcApi;
use bitcoind::BitcoinD;
use crossbeam_channel::{unbounded, Receiver};
use dashmap::DashMap;
use rand::{rng, Rng};
use tracing::{error, info};
use crate::data_generator::generate_random_simulated_transaction;
use crate::data_types::{RandomTx, Wallet};

pub fn main_loop(wallets: Vec<Wallet>, bitcoind: BitcoinD) {
    let cl = &bitcoind.client;

    let (sender, receiver) = unbounded();

    let num_workers = env::var("consumer_threads")
        .unwrap_or("5".to_string()).parse::<u32>().unwrap();
    info!("processing transactions using {:?} threads.", num_workers);
    let mut receiver_handles: Vec<(Receiver<RandomTx>, u32)> = Vec::new();
    receiver_handles.push((receiver.clone(), 0));
    for i in 1..num_workers {
        receiver_handles.push((receiver.clone(), i));
    }

    let map: DashMap<Address, Amount> = DashMap::new();

    let unique_ids = Arc::new(RwLock::new(HashSet::new()));

    // initialize map with the balances of wallets
    for wallet in &wallets {
        let balance = cl.get_balances().unwrap();
        map.insert(wallet.address.clone(), balance.mine.trusted);
    }

    let num_valid_transactions = AtomicU16::new(0);
    let receiver_down = AtomicU8::new(0);

    let num_transactions = env::var("num_transactions")
        .unwrap_or("0".to_string()).parse::<u32>().unwrap();
    let limit = num_transactions != 0u32;

    let base_tx_time = env::var("estimated_tx_time_in_millis")
        .unwrap_or("300".to_string()).parse::<u64>().unwrap();
    let scale: f32 = rng().random_range(0.9..1.1);
    let tx_time: u64 = (base_tx_time as f32 * scale) as u64;


    thread::scope(
        |scope| {
            scope.spawn( || {
                let mut rng = rng();
                let mut cnt = 0;
                loop {
                    let tx_id = generate_random_simulated_transaction(&wallets, &mut rng);
                    sender.send(tx_id).unwrap();
                    thread::sleep(Duration::from_millis(tx_time));
                    if limit && cnt > num_transactions {
                        break;
                    }
                    cnt += 1;
                }
            });

            for receiver_data in &receiver_handles {
                scope.spawn( || {
                    loop {
                        info!("thread_index: {:?}", receiver_data.1);
                        let x = receiver_data.0.recv_timeout(Duration::from_secs(5));
                        match x {
                            Ok(random_tx) => {
                                info!("random_tx: {:?}", random_tx);
                                // now using addresses of sender and receiver, find the wallet and
                                // then using rpc calls, check the balance, positivity of amount, etc
                                // also check the uuid to prevent double spending
                                let sender_address = &random_tx.sender;
                                let receiver_address = &random_tx.receiver;
                                let amount = &random_tx.amount;
                                if amount.le(&Amount::ZERO) {
                                    error!("invalid amount");
                                    continue;
                                }
                                let unique_ids_clone = Arc::clone(&unique_ids);
                                let unique_ids = unique_ids_clone.read().unwrap();
                                if unique_ids.contains(&random_tx.unique_id) {
                                    info!("no way to double spending!!");
                                    continue;
                                }
                                drop(unique_ids);
                                unique_ids_clone.write().unwrap().insert(random_tx.unique_id);
                                let index_sender = wallets.iter()
                                    .position(|w| w.address.eq(sender_address));
                                let index_receiver = wallets.iter()
                                    .position(|w| w.address.eq(receiver_address));
                                if index_sender.is_none() || index_receiver.is_none() {
                                    error!("invalid addresses for sender/receiver!");
                                    continue;
                                }
                                let index_sender = index_sender.unwrap();
                                let index_receiver = index_receiver.unwrap();

                                let send_result = wallets[index_sender].client.send_to_address(
                                    &wallets[index_receiver].address,
                                    *amount,
                                    None, None, None, None, None, None);
                                if send_result.is_err() {
                                    error!("insufficient balance!");
                                    continue;
                                }
                                let balance_sender = wallets[index_sender].client.get_balances().unwrap();
                                map.insert(sender_address.clone(), balance_sender.mine.trusted);

                                let mut receiver_current = map.get_mut(receiver_address).unwrap();
                                *receiver_current += *amount;

                                num_valid_transactions.fetch_add(1, Relaxed);
                            }
                            Err(_) => {
                                receiver_down.fetch_add(1, Relaxed);
                                break;
                            }
                        }
                    }
                });
            }

            scope.spawn(|| {
                let mut total = 0;
                let start = Instant::now();
                loop {
                    thread::sleep(Duration::from_millis(1));
                    let num_valid_tx = num_valid_transactions.load(Relaxed);
                    if num_valid_tx > 50 {
                        total += num_valid_tx;
                        let throughput = total as f64 / start.elapsed().as_secs_f64();
                        info!("avg throughput so far: {}", throughput);
                        num_valid_transactions.store(0, Relaxed);
                        let block_hash = cl.generate_to_address(1, &wallets[0].address).unwrap();
                        info!("blockhash: {:?}", block_hash);
                        let bb = cl.get_block(block_hash.last().unwrap()).unwrap();
                        info!("block tx len: {:?}", bb.txdata.len());
                    }
                    if receiver_down.load(Relaxed) == receiver_handles.len() as u8 {
                        break;
                    }
                }
            });
        }
    );
}
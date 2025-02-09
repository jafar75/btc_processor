# btc_processor
A simplified Bitcoin transaction processor in Rust. The project is developed as a multi-threaded system that crafting simulated transactions and validating/processing them are all done in different threads concurrently.

## Build and Run
in local, simply run the command below:

`cargo run --release`

there is also a minimal docker-compose for the project:

`docker-compose up --build`

## Some assumptions
The main part of the projects consists of one thread to craft the transactions and multiple other threads to consume and process them. At first, I decided to create transactions and then use `sendtoaddress` in order to send them to the regtest nework. But I noticed that this rpc, will validate the transaction `itself` (e.g. rasing errors like Insufficient funds) and there will no room for me to validate the transactions. So I changed my mind and I manually craft the transactions using a data type I defined called `RandomTx` and send them to receiver threads. in these threads, I can do some validations and in order to check the balance, I use the 'sendtoaddress` and by checking the result of the call, I can go to next steps.

## notes about concurrency primitives
I used various types of concurrency primitives in the project. to check the uniqueness of the transactions, I used `RwLock` and `Arc` to pass a `HashSet` between threads. Using `Arc` is for the lock itself, in order to have a reference in each thread. generally using `RwLock` will give a better performance in compare with `Mutex`, as the first one, only locks for write and many readers can do their job without lokcing each other.

To obtain some necessary information and logs, I leveraged `Atomic` variables. e.g. to find the total transactions that successfully validated, I used a `AtomicU16`.

For spawning the threads, I used `thread::scope` in Rust std, as it's the excellent way, when we don't want to `move` everything we are using inside a thread. by using scope, we can use data structures as reference in many threads we want. If we want to spawn a thread normally in Rust, (by using `thread::spawn`), it will have a `'static` lifetime, as Rust assumes the thread will long to end of the program and on that case we MUST `move` everything (even for read-only) to the thread.

## external crates
**crossbeam-channel**: I used it for building a spmc channel, (single producer, multiple consumers). If I want to use the std Rust channels, I had to `move` them and other variables to each thread.

**bitcoind**: A good rpc client for bitcoin regtest.

**rand**: to generate random numbers for different cases.

**uuid**: when I create simulated transactions, I use this crate to generate unique ids and in receiver threads, I check the uuid to prevent double spending. 

**dashmap**: A fast concurrent HashMap. I used it to store and maintain ledger of balances and update it in a multi-threaded environment. I used this crate in a real-world project and it is really fast. I want to mention, I could have not used this crate and by `RwLock` I could get the same functional results. 

## future works
- By using the methods like [this](https://bitcoin.stackexchange.com/questions/28182/how-to-find-the-change-sender-address-given-a-txid), one can find the sender of a transaction. Maybe, it is better to use `createrawtransaction` in the sender side and in the receiver, by using the link, find the sender wallet, then sign it and send it. it is more advanced than `snedtoaddress`, and gives more control to the selection of UTXOs to spend.

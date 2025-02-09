# First stage: Build the Rust app
FROM rust:1.83 as builder

WORKDIR /app

ADD . /app
RUN ls .
RUN cargo build --release --jobs 6

CMD ["/app/target/release/btc_processor"]


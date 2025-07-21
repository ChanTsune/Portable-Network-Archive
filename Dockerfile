FROM rust:slim as dev

ENV CARGO_TARGET_DIR /tmp/target/

RUN rustup component add clippy rustfmt

RUN apt update && apt install -y libacl1-dev g++ cmake git bats shfmt shellcheck

FROM ghcr.io/portable-network-archive/wasi-sdk-gh-actions:wasi-sdk-25 as wasi

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    CARGO_TARGET_DIR=/tmp/target/

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain none --profile minimal -y


RUN curl https://wasmtime.dev/install.sh -sSf | bash

RUN rustup default nightly && \
    rustup component add clippy rustfmt && \
    rustup target add wasm32-unknown-unknown wasm32-wasip1 wasm32-wasip2

FROM rust:slim as builder

RUN rustup target add "$(uname -m)"-unknown-linux-musl

RUN apt update && apt install -y libacl1-dev cmake musl-tools

WORKDIR /work

ENV CARGO_TARGET_DIR /tmp/target/

RUN --mount=type=bind,target=. cargo build -p portable-network-archive --all-features --release --locked --target "$(uname -m)"-unknown-linux-musl

RUN strip "$CARGO_TARGET_DIR$(uname -m)"-unknown-linux-musl/release/pna -o /pna

FROM scratch as binary

COPY --from=builder /pna /

ENTRYPOINT ["/pna"]

FROM rust:slim as dev

ENV CARGO_TARGET_DIR /tmp/target/

RUN rustup component add clippy rustfmt

RUN apt update && apt install -y libacl1-dev g++ cmake git bats shfmt shellcheck

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

FROM rust:slim as builder

RUN rustup target add "$(uname -m)"-unknown-linux-musl

RUN apt update && apt install -y libacl1-dev musl-tools

WORKDIR /work

COPY . .

RUN cargo build -p portable-network-archive --all-features --release --locked --target "$(uname -m)"-unknown-linux-musl

RUN strip /work/target/"$(uname -m)"-unknown-linux-musl/release/pna -o /pna

FROM scratch as binary

COPY --from=builder /pna /

ENTRYPOINT ["/pna"]

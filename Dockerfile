# https://dev.to/deciduously/use-multi-stage-docker-builds-for-statically-linked-rust-binaries-3jgd

# Build Stage
FROM rust AS builder
WORKDIR /usr/src/
RUN rustup target add x86_64-unknown-linux-musl

RUN USER=root cargo new asyncfibber
WORKDIR /usr/src/asyncfibber
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Bundle Stage
FROM scratch
COPY --from=builder /usr/local/cargo/bin/asyncfibber .
USER 1000
EXPOSE 1234
CMD ["./asyncfibber"]

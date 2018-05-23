FROM rust:1-stretch as builder
WORKDIR /usr/src/mtproxy
COPY . .
RUN cargo build --release

FROM debian:stretch-slim
COPY --from=builder /usr/src/mtproxy/target/release/mtproxy /bin/

ENTRYPOINT ["mtproxy"]
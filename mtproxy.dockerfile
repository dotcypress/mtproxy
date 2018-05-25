FROM ekidd/rust-musl-builder as builder
WORKDIR /home/rust/src
ADD . ./
RUN sudo chown -R rust:rust .
RUN cargo build --release

FROM alpine:latest
COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/mtproxy /bin/

ENTRYPOINT ["mtproxy"]
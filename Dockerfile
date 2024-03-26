FROM rust:alpine3.18 as builder

WORKDIR /app
RUN apk add --no-cache musl-dev openssl-dev openssl protoc gcc
ENV OPENSSL_DIR=/usr
# ENV RUSTFLAGS=-C target-feature=-crt-static

COPY . .
RUN cargo build --release

FROM alpine:3.19 as runner

RUN apk add --no-cache musl-dev openssl-dev openssl protoc gcc
ENV OPENSSL_DIR=/usr

WORKDIR /app
COPY --from=builder /app/target/release/tapoctl /usr/bin/tapoctl

CMD [ "tapoctl", "serve" ]

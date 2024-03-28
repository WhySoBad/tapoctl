FROM rust:alpine3.18 as builder

WORKDIR /app
RUN apk add --no-cache musl-dev openssl-dev openssl protoc gcc
ENV OPENSSL_DIR=/usr
ENV RUSTFLAGS=-Ctarget-feature=-crt-static
ENV CARGO_BUILD_TARGET=x86_64-unknown-linux-musl
ENV OPENSSL_NO_VENDOR=1

COPY . .
RUN cargo build --release

FROM alpine:3.19 as runner

RUN apk add --no-cache musl-dev openssl-dev openssl protoc gcc
ENV OPENSSL_DIR=/usr
ENV OPENSSL_NO_VENDOR=1

RUN addgroup --system --gid 1001 tapo
RUN adduser --system --uid 1001 tapo

WORKDIR /home/tapo

COPY --from=builder --chown=tapo:tapo /app/target/x86_64-unknown-linux-musl/release/tapoctl /usr/bin/tapoctl

USER tapo

EXPOSE 19191

CMD [ "tapoctl", "serve" ]
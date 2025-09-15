FROM rust:alpine3.22 AS builder
ARG TARGETPLATFORM

WORKDIR /app

RUN rustup target add x86_64-unknown-linux-musl
RUN rustup target add aarch64-unknown-linux-musl

RUN apk add --no-cache musl-dev openssl-dev openssl protoc gcc
ENV OPENSSL_DIR=/usr
ENV RUSTFLAGS=-Ctarget-feature=-crt-static
ENV OPENSSL_NO_VENDOR=1

RUN CARGO_BUILD_TARGET=$([ "$TARGETPLATFORM" = "arm" ] && echo "aarch64-unknown-linux-musl" || echo "x86_64-unknown-linux-musl")

COPY . .
RUN cargo build --release

FROM alpine:3.22 AS runner
ARG TARGETPLATFORM

RUN apk add --no-cache musl-dev openssl-dev openssl protoc gcc
ENV OPENSSL_DIR=/usr
ENV OPENSSL_NO_VENDOR=1

RUN addgroup --system --gid 1001 tapo
RUN adduser --system --uid 1001 tapo

WORKDIR /home/tapo

COPY --from=builder --chown=tapo:tapo /app/target/release/tapoctl /usr/bin/tapoctl

USER tapo

EXPOSE 19191

CMD [ "tapoctl", "serve" ]
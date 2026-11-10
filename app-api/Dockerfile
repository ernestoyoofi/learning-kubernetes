# syntax=docker/dockerfile:1
FROM --platform=$BUILDPLATFORM tonistiigi/xx:1.10.0 AS xx

FROM --platform=$BUILDPLATFORM rust:1.90-alpine AS builder

COPY --from=xx / /

ARG TARGETPLATFORM

RUN apk add --no-cache musl-dev clang lld
RUN xx-apk add --no-cache musl-dev gcc

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    xx-cargo build --release && \
    rm -rf src

COPY . .
RUN touch src/main.rs && xx-cargo build --release

RUN TRIPLE=$(xx-cargo --print-target-triple) && \
    cp target/$TRIPLE/release/app-api /app-api && \
    xx-verify --static /app-api

FROM scratch

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app-api /app-api

EXPOSE 8080

ENTRYPOINT ["/app-api"]

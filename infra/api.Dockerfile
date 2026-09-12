# syntax=docker/dockerfile:1.27.0@sha256:bde3983e9c939224420ddaf6b784cc30e09b035a4dea01f581230c50809f372e
FROM rust:1.98.1-alpine@sha256:1716b3aa042d735f4566d14dc54e8037de9d69556e2d5dd58131d93a613d173d AS builder
ENV CARGO_INCREMENTAL=0
WORKDIR /app
RUN apk add --no-cache \
    musl musl-dev libc-dev build-base \
    lld mold cmake clang clang-dev \
    openssl-dev pkgconfig git curl
RUN rustup target add x86_64-unknown-linux-musl
RUN --mount=type=bind,source=.,target=/src \
    --mount=type=cache,id=rokbattles-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=rokbattles-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=rokbattles-cargo-target-x86_64-musl,target=/target,sharing=locked \
    cd /src && \
    CARGO_TARGET_DIR=/target cargo build --profile server --locked --target x86_64-unknown-linux-musl -p rokbattles-api && \
    cp /target/x86_64-unknown-linux-musl/server/rokbattles-api /app/rokbattles-api

FROM alpine:3.24@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b AS files
RUN apk add --no-cache ca-certificates tzdata
RUN addgroup --system --gid 10001 rokb && \
    adduser  --system --uid 10001 --ingroup rokb --home /nonexistent --shell /sbin/nologin rokb
RUN update-ca-certificates

FROM scratch AS runner
COPY --link --from=files /etc/passwd /etc/group /etc/nsswitch.conf /etc/
COPY --link --from=files /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --link --from=files /usr/share/zoneinfo /usr/share/zoneinfo
COPY --link --from=builder /app/rokbattles-api /bin/rokbattles-api
USER rokb:rokb
WORKDIR /app
EXPOSE 8001
ENTRYPOINT ["/bin/rokbattles-api"]

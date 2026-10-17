# syntax=docker/dockerfile:1.28.0@sha256:bb22d9815c728170f72750f4e5b0d672e06176142e1d602c7e66c050100b7e5b
FROM rust:1.99.1-alpine@sha256:dc578ff3ca62ef489ad9ae743f477b94d98739c3092d0023490ecd417c2eebea AS builder
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
    CARGO_TARGET_DIR=/target cargo build --profile server --locked --target x86_64-unknown-linux-musl -p rokbattles-dns-resolver && \
    cp /target/x86_64-unknown-linux-musl/server/rokbattles-dns-resolver /app/rokbattles-dns-resolver

FROM alpine:3.24.2@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6 AS files
RUN apk add --no-cache ca-certificates tzdata
RUN addgroup --system --gid 10001 rokb && \
    adduser  --system --uid 10001 --ingroup rokb --home /nonexistent --shell /sbin/nologin rokb
RUN update-ca-certificates

FROM scratch AS runner
COPY --link --from=files /etc/passwd /etc/group /etc/nsswitch.conf /etc/
COPY --link --from=files /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --link --from=files /usr/share/zoneinfo /usr/share/zoneinfo
COPY --link --from=builder /app/rokbattles-dns-resolver /bin/rokbattles-dns-resolver
USER rokb:rokb
WORKDIR /app
EXPOSE 8053
ENTRYPOINT ["/bin/rokbattles-dns-resolver"]

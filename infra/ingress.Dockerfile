# syntax=docker/dockerfile:1
FROM rust:1.97.1-alpine@sha256:3c38f3f82c2f3d73da3b38e18d279393a04cb43ddded0e35088a8c3324d40900 AS builder
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
    CARGO_TARGET_DIR=/target cargo build --release --locked --target x86_64-unknown-linux-musl -p rokbattles-ingress && \
    cp /target/x86_64-unknown-linux-musl/release/rokbattles-ingress /app/rokbattles-ingress

FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce AS files
RUN apk add --no-cache ca-certificates tzdata
RUN addgroup --system --gid 10001 rokb && \
    adduser  --system --uid 10001 --ingroup rokb --home /nonexistent --shell /sbin/nologin rokb
RUN update-ca-certificates

FROM scratch AS runner
COPY --link --from=files /etc/passwd /etc/group /etc/nsswitch.conf /etc/
COPY --link --from=files /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --link --from=files /usr/share/zoneinfo /usr/share/zoneinfo
COPY --link --from=builder /app/rokbattles-ingress /bin/rokbattles-ingress
USER rokb:rokb
WORKDIR /app
EXPOSE 8000
ENTRYPOINT ["/bin/rokbattles-ingress"]

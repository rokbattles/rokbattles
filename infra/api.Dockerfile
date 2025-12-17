FROM rust:1.92-alpine AS builder
WORKDIR /app
RUN apk add --no-cache \
    musl musl-dev libc-dev build-base \
    lld mold cmake clang clang-dev \
    openssl-dev pkgconfig git curl
RUN rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl -p rokbattles-api

FROM alpine:3.20 AS files
RUN apk add --no-cache ca-certificates tzdata
RUN addgroup --system --gid 10001 rokb && \
    adduser  --system --uid 10001 --ingroup rokb --home /nonexistent --shell /sbin/nologin rokb
RUN update-ca-certificates

FROM scratch AS runner
COPY --from=files /etc/passwd /etc/passwd
COPY --from=files /etc/group /etc/group
COPY --from=files /etc/nsswitch.conf /etc/nsswitch.conf
COPY --from=files /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=files /usr/share/zoneinfo /usr/share/zoneinfo
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rokbattles-api /bin/rokbattles-api
USER rokb:rokb
WORKDIR /app
EXPOSE 8000
ENTRYPOINT ["/bin/rokbattles-api"]

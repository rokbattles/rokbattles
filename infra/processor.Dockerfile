FROM rust:1.97.1-alpine@sha256:3c38f3f82c2f3d73da3b38e18d279393a04cb43ddded0e35088a8c3324d40900 AS builder
ENV CARGO_INCREMENTAL=0
WORKDIR /app
RUN apk add --no-cache \
    musl musl-dev libc-dev build-base \
    lld mold cmake clang clang-dev \
    openssl-dev pkgconfig git curl
RUN rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --locked --target x86_64-unknown-linux-musl -p rokbattles-processor

FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce AS files
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
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rokbattles-processor /bin/rokbattles-processor
USER rokb:rokb
WORKDIR /app
ENTRYPOINT ["/bin/rokbattles-processor"]

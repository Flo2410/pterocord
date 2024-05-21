FROM rust:1.78-bookworm as builder

WORKDIR /app
COPY . .

RUN cargo build --release --locked

FROM rust:1.78-slim-bookworm
WORKDIR /app

COPY --from=builder /app/target/release/pterocord /app

CMD ["/app/pterocord"]

LABEL org.opencontainers.image.authors "Florian Hye <florian@hye.dev>"
LABEL org.opencontainers.image.version "v1.0.0"
LABEL org.opencontainers.image.source "https://github.com/flo2410/pterocord"
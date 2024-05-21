FROM rust:1.78-bookworm as builder

WORKDIR /app
COPY . .

RUN cargo build --release --locked

FROM rust:1.78-slim-bookworm
WORKDIR /app

COPY --from=builder /app/target/release/pterocord /app

CMD ["/app/pterocord"]
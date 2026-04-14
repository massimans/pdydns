FROM rust:1.94-bookworm AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/release/pdydns /usr/local/bin/pdydns

USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/pdydns"]

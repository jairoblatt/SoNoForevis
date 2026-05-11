FROM rust:slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && RUSTFLAGS="-C target-cpu=haswell" cargo build --release \
    && rm -f target/release/deps/SoNoForevis*
COPY src ./src
RUN RUSTFLAGS="-C target-cpu=haswell" cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/SoNoForevis /proxy
ENTRYPOINT ["/proxy"]

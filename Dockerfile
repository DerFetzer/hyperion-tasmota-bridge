# Create the build container to compile the hello world program
FROM rust:1.58.1-buster as builder
RUN apt-get update && apt-get install -y cmake musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /hyperion-tasmota-bridge
COPY . .
RUN cargo build --target=x86_64-unknown-linux-musl --release

# Create the execution container by copying the compiled hello world to it and running it
FROM scratch
WORKDIR /hyperion-tasmota-bridge
COPY --from=builder /hyperion-tasmota-bridge/target/x86_64-unknown-linux-musl/release/hyperion-tasmota-bridge /hyperion-tasmota-bridge/hyperion-tasmota-bridge
CMD ["/hyperion-tasmota-bridge/hyperion-tasmota-bridge"]
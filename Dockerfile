# --- Build stage ---
FROM rust:latest AS build-stage

WORKDIR /opt/rusk
ENV RUSK_PROFILE_PATH /opt/rusk
ENV DUSK_CONSENSUS_KEYS_PASS password
# Expose necessary ports (assuming 9000 for Kadcast's UDP)
EXPOSE 9000/udp

RUN apt-get update && apt-get install -y clang && rm -rf /var/lib/apt/lists/*

COPY . .

# Install specified nightly
RUN rustup toolchain install nightly-2023-05-22-x86_64-unknown-linux-gnu && \
    rustup component add rust-src --toolchain nightly-2023-05-22-x86_64-unknown-linux-gnu

# Generate keys, compile genesis contracts and  generate genesis state
RUN make keys && make wasm
RUN mkdir -p ~/.dusk/rusk && cp examples/consensus.keys ~/.dusk/rusk/consensus.keys
RUN cargo r --release -p rusk-recovery --features state --bin rusk-recovery-state -- --init examples/genesis.toml -o /tmp/example.state
RUN cargo b --release -p rusk

# --- Run stage ---
FROM debian:bookworm-slim

WORKDIR /opt/rusk

ENV RUSK_PROFILE_PATH /opt/rusk/
ENV DUSK_CONSENSUS_KEYS_PASS password
EXPOSE 9000/udp

# Copy only the necessary files from the build stage
COPY --from=build-stage /opt/rusk/.rusk /opt/rusk/.rusk
COPY --from=build-stage /opt/rusk/target/release/rusk /opt/rusk/
COPY --from=build-stage /opt/rusk/target/release/rusk-recovery-keys /opt/rusk/
COPY --from=build-stage /opt/rusk/target/release/rusk-recovery-state /opt/rusk/
COPY --from=build-stage /opt/rusk/examples/consensus.keys /opt/rusk/consensus.keys
COPY --from=build-stage /tmp/example.state /tmp/example.state

CMD ["./rusk", "-s", "/tmp/example.state", "--consensus-keys-path", "/opt/rusk/consensus.keys"]

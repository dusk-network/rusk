# --- Build stage ---
FROM rust:latest AS build-stage

WORKDIR /opt/rusk
ENV RUSK_PROFILE_PATH /.dusk/rusk
ENV DUSK_CONSENSUS_KEYS_PASS password
# Expose necessary ports (assuming 9000 for Kadcast's UDP)
EXPOSE 9000/udp

RUN apt-get update && apt-get install -y clang && rm -rf /var/lib/apt/lists/*

COPY . .

ARG TARGETPLATFORM
# Convert Docker platform arg to Rust target name,
# and install nightly based on the Rust target
RUN ARCH="$(echo $TARGETPLATFORM | sed 's/linux\///')" && \
    case "$ARCH" in \
    "amd64") RUST_ARCH="x86_64";; \
    "arm64") RUST_ARCH="aarch64";; \
    "arm/v7") RUST_ARCH="armv7";; \
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;; \
    esac && \
    RUST_TARGET="$RUST_ARCH-unknown-linux-gnu" && \
    echo "Rust target: $RUST_TARGET" && \
    rustup toolchain install nightly-2023-05-22-$RUST_TARGET && \
    rustup component add rust-src --toolchain nightly-2023-05-22-$RUST_TARGET

# Generate keys, compile genesis contracts and generate genesis state
RUN make keys && make wasm
RUN mkdir -p /.dusk/rusk && cp examples/consensus.keys /.dusk/rusk/consensus.keys
RUN cargo r --release -p rusk -- recovery-state --init examples/genesis.toml -o /tmp/example.state
RUN cargo b --release -p rusk

# --- Run stage ---
FROM debian:bookworm-slim

WORKDIR /opt/rusk

ENV RUSK_PROFILE_PATH /.dusk/rusk/
ENV DUSK_CONSENSUS_KEYS_PASS password
EXPOSE 9000/udp

RUN apt-get update && apt-get install -y libssl-dev  && rm -rf /var/lib/apt/lists/*

# Copy only the necessary files from the build stage
COPY --from=build-stage /.dusk/rusk /.dusk/rusk
COPY --from=build-stage /opt/rusk/target/release/rusk /opt/rusk/
COPY --from=build-stage /opt/rusk/examples/consensus.keys /opt/rusk/consensus.keys
COPY --from=build-stage /tmp/example.state /tmp/example.state

CMD ["./rusk", "-s", "/tmp/example.state", "--consensus-keys-path", "/opt/rusk/consensus.keys"]
# --- Build stage ---
FROM rust:latest AS build-stage

WORKDIR /opt/rusk
ENV RUSK_PROFILE_PATH /.dusk/rusk

RUN apt-get update && apt-get install -y clang && rm -rf /var/lib/apt/lists/*

COPY . .

ARG TARGETPLATFORM
# See also https://github.com/docker/buildx/issues/510
ENV TARGETPLATFORM=${TARGETPLATFORM:-linux/amd64}

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

# Generate keys and compile genesis contracts
RUN make keys
RUN make wasm
RUN cargo b --release -p rusk

# --- Run stage ---
FROM debian:bookworm-slim

WORKDIR /opt/rusk

ENV RUSK_PROFILE_PATH /.dusk/rusk/
ENV DUSK_CONSENSUS_KEYS_PASS password
EXPOSE 9000/udp
EXPOSE 8080/tcp

RUN apt-get update && apt-get install -y libssl-dev  && rm -rf /var/lib/apt/lists/*

# Copy only the necessary files from the build stage
COPY --from=build-stage /.dusk/rusk /.dusk/rusk
COPY --from=build-stage /opt/rusk/target/release/rusk /opt/rusk/
COPY --from=build-stage /opt/rusk/examples/consensus.keys /opt/rusk/consensus.keys
COPY --from=build-stage /opt/rusk/examples/genesis.toml /opt/rusk/state.toml

CMD ./rusk recovery-state --init /opt/rusk/state.toml -o /tmp/state; ./rusk -s /tmp/state --consensus-keys-path /opt/rusk/consensus.keys --http-listen-addr 0.0.0.0:8080

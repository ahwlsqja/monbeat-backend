# === Stage 1: C++ Engine Build (from Docker Hub) ===
FROM ahwlsqja/monad-vibe-cli:latest AS cpp-engine

# === Stage 2: Rust Builder ===
FROM rust:1.94-slim-bookworm AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace manifests first (cache layer)
COPY Cargo.toml Cargo.lock ./
COPY crates/types/Cargo.toml crates/types/Cargo.toml
COPY crates/state/Cargo.toml crates/state/Cargo.toml
COPY crates/evm/Cargo.toml crates/evm/Cargo.toml
COPY crates/precompiles/Cargo.toml crates/precompiles/Cargo.toml
COPY crates/nine-fork/Cargo.toml crates/nine-fork/Cargo.toml
COPY crates/mv-state/Cargo.toml crates/mv-state/Cargo.toml
COPY crates/scheduler/Cargo.toml crates/scheduler/Cargo.toml
COPY crates/cli/Cargo.toml crates/cli/Cargo.toml
COPY crates/monbeat-server/Cargo.toml crates/monbeat-server/Cargo.toml

# Stub out lib.rs for each crate so cargo can resolve the dependency graph
RUN for crate_dir in types state evm precompiles nine-fork mv-state scheduler cli monbeat-server; do \
      mkdir -p "crates/${crate_dir}/src" && \
      echo "" > "crates/${crate_dir}/src/lib.rs"; \
    done && \
    echo "fn main() {}" > crates/monbeat-server/src/main.rs

# Pre-fetch and compile dependencies (cached unless Cargo.toml changes)
RUN cargo build --release -p monbeat-server 2>&1 || true

# Now copy the real source
COPY crates/ crates/

# Touch source files so cargo sees them as newer than the stubs
RUN find crates/ -name "*.rs" -exec touch {} +

# Build the actual binary
RUN cargo build --release -p monbeat-server

# === Stage 3: Runtime ===
# Use Ubuntu 25.10 to match C++ engine's shared library requirements
FROM ubuntu:25.10

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3t64 wget \
    # C++ engine shared libs (must match monad-vibe-cli build)
    libboost-fiber1.83.0 \
    libboost-json1.83.0 \
    libboost-stacktrace1.83.0 \
    libtbb12 \
    libzstd1 \
    libgmp10 \
    liburing2 \
    libbrotli1 \
    libcrypto++8 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install solc 0.8.28 static binary
RUN wget -q -O /usr/local/bin/solc \
    "https://github.com/ethereum/solidity/releases/download/v0.8.28/solc-static-linux" \
    && chmod +x /usr/local/bin/solc \
    && solc --version

# Copy C++ engine binary from pre-built image
COPY --from=cpp-engine /usr/local/bin/monad-vibe-cli /usr/local/bin/monad-vibe-cli

# Copy Rust server binary from builder
COPY --from=rust-builder /app/target/release/monbeat-server /usr/local/bin/monbeat-server

# Copy migrations
COPY crates/monbeat-server/migrations/ /app/migrations/

ENV PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

CMD ["monbeat-server"]

# === Stage 1: Builder ===
FROM rust:1.84-slim-bookworm AS builder

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
    # monbeat-server has a binary target too
    echo "fn main() {}" > crates/monbeat-server/src/main.rs

# Pre-fetch and compile dependencies (cached unless Cargo.toml changes)
RUN cargo build --release -p monbeat-server 2>&1 || true

# Now copy the real source
COPY crates/ crates/

# Touch source files so cargo sees them as newer than the stubs
RUN find crates/ -name "*.rs" -exec touch {} +

# Build the actual binary
RUN cargo build --release -p monbeat-server

# === Stage 2: Runtime ===
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 wget \
    && rm -rf /var/lib/apt/lists/*

# Install solc 0.8.28 static binary
RUN wget -q -O /usr/local/bin/solc \
    "https://github.com/ethereum/solidity/releases/download/v0.8.28/solc-static-linux" \
    && chmod +x /usr/local/bin/solc \
    && solc --version

# Copy the compiled binary from builder
COPY --from=builder /app/target/release/monbeat-server /usr/local/bin/monbeat-server

# Copy migrations (used at runtime via include_str! — already embedded, but kept for reference)
COPY crates/monbeat-server/migrations/ /app/migrations/

ENV PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

CMD ["monbeat-server"]

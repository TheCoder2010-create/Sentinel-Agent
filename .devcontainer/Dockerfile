# ---------------------------------------------------------------------------
# Sentinel AI — Dev Container base image
#
# Builds on ubuntu:noble with:
#   - Rust toolchain (stable) + musl cross-compilation targets
#   - clippy, rustfmt, rust-analyzer
#   - Python 3.12 + pipx
#   - Node.js 22 + npm
#   - Bazelisk (Bazel launcher)
#   - bubblewrap (for sandbox execution)
#   - iptables + ipset (for secure profile firewall)
#   - Essential C/C++ build tools + musl-tools
# ---------------------------------------------------------------------------
FROM ubuntu:noble

ENV DEBIAN_FRONTEND=noninteractive
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=/usr/local/cargo/bin:/usr/local/bin:$PATH

SHELL ["/bin/bash", "-c"]

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        # Build essentials
        build-essential \
        pkg-config \
        cmake \
        ninja-build \
        make \
        gcc \
        g++ \
        # musl cross-compilation
        musl-tools \
        clang \
        llvm \
        lld \
        # Networking & firewall
        iptables \
        ipset \
        iproute2 \
        nftables \
        dnsutils \
        curl \
        ca-certificates \
        # Sandbox
        bubblewrap \
        # Git
        git \
        git-lfs \
        # General utilities
        jq \
        unzip \
        xz-utils \
        gnupg \
        lsb-release \
        software-properties-common \
        wget \
        # Node.js repo setup
        && mkdir -p /etc/apt/keyrings \
        && curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg \
        && echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_22.x noble main" > /etc/apt/sources.list.d/nodesource.list \
        # Python
        && add-apt-repository -y ppa:deadsnakes/ppa \
        && apt-get update; \
    apt-get install -y --no-install-recommends \
        nodejs \
        python3.12 \
        python3.12-dev \
        python3.12-venv \
        python3-pip \
        pipx; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*

# --- Rust toolchain (stable) ---
RUN set -eux; \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --default-toolchain stable \
        --profile complete \
        --no-modify-path; \
    rustup target add \
        x86_64-unknown-linux-musl \
        aarch64-unknown-linux-musl \
        wasm32-unknown-unknown; \
    rustup component add clippy rustfmt rust-analyzer; \
    cargo install cargo-audit cargo-deny; \
    rm -rf /usr/local/cargo/registry

# --- Bazelisk (Bazel launcher) ---
RUN set -eux; \
    curl -fsSL https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64 -o /usr/local/bin/bazel; \
    chmod +x /usr/local/bin/bazel

# --- Cleanup ---
RUN set -eux; \
    rustup cache clean 2>/dev/null || true; \
    rm -rf /tmp/*

# --- Default user ---
RUN useradd -m -s /bin/bash vscode && \
    usermod -aG sudo vscode && \
    echo "vscode ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

USER vscode
WORKDIR /workspaces/sentinel

CMD ["sleep", "infinity"]

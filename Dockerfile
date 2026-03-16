FROM rust:latest

# Install Python 3, git, curl, and GitHub CLI
RUN apt-get update && apt-get install -y python3 git curl && \
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
        | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] \
        https://cli.github.com/packages stable main" \
        | tee /etc/apt/sources.list.d/github-cli.list > /dev/null && \
    apt-get update && apt-get install -y gh && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Pre-warm the cargo dependency cache so evolution builds are fast.
# This layer is rebuilt only when Cargo.toml / Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release && \
    rm -rf src target

# Source code is bind-mounted at runtime — not copied here.
# This lets the agent modify its own source and have changes persist on the host.
CMD ["bash", "./scripts/evolve.sh"]

#!/bin/bash

# Check if Rust is installed by trying to run 'rustc'
if ! command -v rustc &> /dev/null
then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    cargo update
else
    echo "Rust is already installed. Skipping installation."
fi

# Continue with build process
cargo build --release && cargo install --path .

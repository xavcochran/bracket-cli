#!/bin/bash

# Set your repository and version
REPO="bracketengineering/bracket-cli"
VERSION="latest" # or specify a version like "v1.0.0"

# Detect the operating system
OS=$(uname -s)
ARCH=$(uname -m)

LINUX="ubuntu-latest"
MACOS="macos-latest"

AMD64="x86_64"
ARM64="arm64"

# Determine the appropriate binary to download
if [[ "$OS" == "Linux" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        FILE=$LINUX + $AMD64
    elif [[ "$ARCH" == "arm64" ]]; then
        FILE=$LINUX + $ARM64
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
elif [[ "$OS" == "Darwin" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        FILE=$MACOS + $AMD64
    elif [[ "$ARCH" == "arm64" ]]; then
        FILE=$MACOS + $ARM64
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
else
    echo "Unsupported OS: $OS"
    exit 1
fi

# Download the binary
URL="https://github.com/$REPO/releases/download/$VERSION/$FILE"
curl -L $URL -o /usr/local/bin/bracket

# Make the binary executable
chmod +x /usr/local/bin/bracket

# Verify installation
if command -v bracket >/dev/null 2>&1; then
    echo "Installation successful!"
else
    echo "Installation failed."
    exit 1
fi

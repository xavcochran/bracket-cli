#!/bin/bash

# Set your repository and version
REPO="yourusername/your-repo"
VERSION="latest" # or specify a version like "v1.0.0"

# Detect the operating system
OS=$(uname -s)
ARCH=$(uname -m)

# Determine the appropriate binary to download
if [[ "$OS" == "Linux" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        FILE="your_binary_name-linux-amd64"
    elif [[ "$ARCH" == "arm64" ]]; then
        FILE="your_binary_name-linux-arm64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
elif [[ "$OS" == "Darwin" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        FILE="your_binary_name-macos-amd64"
    elif [[ "$ARCH" == "arm64" ]]; then
        FILE="your_binary_name-macos-arm64"
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
curl -L $URL -o /usr/local/bin/your_binary_name

# Make the binary executable
chmod +x /usr/local/bin/your_binary_name

# Verify installation
if command -v your_binary_name >/dev/null 2>&1; then
    echo "Installation successful!"
else
    echo "Installation failed."
    exit 1
fi

#!/bin/bash

# Set your repository
REPO="bracketengineering/bracket-cli"

# Fetch the latest release version from GitHub API
VERSION=$(curl --silent "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')

if [[ -z "$VERSION" ]]; then
    echo "Failed to fetch the latest version. Please check your internet connection or the repository."
    exit 1
fi

echo "Latest version is $VERSION"

# Detect the operating system
OS=$(uname -s)
ARCH=$(uname -m)

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

# Ensure that $HOME/.local/bin is in the PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "Adding $HOME/.local/bin to PATH"
    export PATH="$HOME/.local/bin:$PATH"
    # Add it to .bashrc or .zshrc for persistence
    if [[ "$SHELL" == *"bash"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
    elif [[ "$SHELL" == *"zsh"* ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"
    fi
fi

# Determine the appropriate binary to download
if [[ "$OS" == "Linux" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        FILE="bracket-cli-linux-amd64"
    elif [[ "$ARCH" == "arm64" ]]; then
        FILE="bracket-cli-linux-arm64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
elif [[ "$OS" == "Darwin" ]]; then
    if [[ "$ARCH" == "x86_64" ]]; then
        FILE="bracket-cli-macos-amd64"
    elif [[ "$ARCH" == "arm64" ]]; then
        FILE="bracket-cli-macos-arm64"
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
echo "Downloading from $URL"
curl -L $URL -o "$INSTALL_DIR/bracket"

# Make the binary executable
chmod +x "$INSTALL_DIR/bracket"

# Verify installation
if command -v bracket >/dev/null 2>&1; then
    echo "Installation successful!"
else
    echo "Installation failed."
    exit 1
fi

echo "Installing dependencies..."
if ! command -v git &> /dev/null
then
    echo "Git is not installed. Installing Git..."
    if [[ "$OS" == "Linux" ]]
    then
        sudo apt update
        sudo apt install git
    elif [[ "$OS" == "Darwin" ]]
    then
        brew install git
    fi
else
    echo "Git is already installed. Skipping installation."
fi

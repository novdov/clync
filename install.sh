#!/bin/bash
set -e

REPO="novdov/claudy"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
OLD_INSTALL_DIR="/usr/local/bin"
API_BASE="https://api.github.com/repos/${REPO}"

if [ -z "$GITHUB_TOKEN" ]; then
    echo "Error: GITHUB_TOKEN environment variable is required"
    echo ""
    echo "Usage:"
    echo "  GITHUB_TOKEN=<token> bash -c \"\$(curl -fsSL -H 'Authorization: token <token>' https://raw.githubusercontent.com/${REPO}/main/install.sh)\""
    echo ""
    echo "Create token: https://github.com/settings/tokens"
    echo "Required scope: repo (private repository access)"
    exit 1
fi

if ! command -v jq &> /dev/null; then
    echo "Error: jq is required"
    echo "Install: brew install jq (macOS) or apt install jq (Linux)"
    exit 1
fi

AUTH_HEADER="Authorization: token ${GITHUB_TOKEN}"

get_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) echo "unsupported" ;;
    esac
}

get_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "x64" ;;
        arm64|aarch64) echo "arm64" ;;
        *) echo "unsupported" ;;
    esac
}

OS=$(get_os)
ARCH=$(get_arch)

if [ "$OS" = "unsupported" ] || [ "$ARCH" = "unsupported" ]; then
    echo "Error: Unsupported platform ($(uname -s) $(uname -m))"
    exit 1
fi

if [ "$OS" = "windows" ]; then
    BINARY_NAME="clync-windows-x64.exe"
    TARGET_NAME="clync.exe"
else
    BINARY_NAME="clync-${OS}-${ARCH}"
    TARGET_NAME="clync"
fi

echo "Installing clync..."
echo "Platform: ${OS}-${ARCH}"

echo "Fetching latest release..."
RELEASE_INFO=$(curl -fsSL -H "$AUTH_HEADER" "${API_BASE}/releases/latest")

VERSION=$(echo "$RELEASE_INFO" | jq -r '.tag_name')
ASSET_URL=$(echo "$RELEASE_INFO" | jq -r ".assets[] | select(.name == \"${BINARY_NAME}\") | .url")

if [ -z "$ASSET_URL" ] || [ "$ASSET_URL" = "null" ]; then
    echo "Error: Binary ${BINARY_NAME} not found"
    echo "Available binaries:"
    echo "$RELEASE_INFO" | jq -r '.assets[].name'
    exit 1
fi

TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

echo "Downloading... (${VERSION})"
curl -fsSL -H "$AUTH_HEADER" -H "Accept: application/octet-stream" "$ASSET_URL" -o "${TMP_DIR}/${TARGET_NAME}"

chmod +x "${TMP_DIR}/${TARGET_NAME}"

if [ -f "${OLD_INSTALL_DIR}/${TARGET_NAME}" ]; then
    echo "Removing old binary from ${OLD_INSTALL_DIR}..."
    sudo rm -f "${OLD_INSTALL_DIR}/${TARGET_NAME}"
fi

mkdir -p "$INSTALL_DIR"
mv "${TMP_DIR}/${TARGET_NAME}" "${INSTALL_DIR}/${TARGET_NAME}"

echo ""
echo "clync ${VERSION} installed to ${INSTALL_DIR}/${TARGET_NAME}"

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo ""
    echo "Add the following to your shell profile (.zshrc, .bashrc, etc.):"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
fi

echo ""
echo "Usage:"
echo "  clync --help"

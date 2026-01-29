#!/bin/bash
set -euo pipefail

# curl-runner installer
# Usage: curl -fsSL https://www.curl-runner.com/install.sh | bash

REPO="alexvcasillas/curl-runner"
INSTALL_DIR="${CURL_RUNNER_INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="curl-runner"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Detect OS
detect_os() {
  local os
  os=$(uname -s | tr '[:upper:]' '[:lower:]')
  case "$os" in
    linux) echo "linux" ;;
    darwin) echo "darwin" ;;
    *) error "Unsupported OS: $os" ;;
  esac
}

# Detect architecture
detect_arch() {
  local arch
  arch=$(uname -m)
  case "$arch" in
    x86_64|amd64) echo "x64" ;;
    aarch64|arm64) echo "arm64" ;;
    *) error "Unsupported architecture: $arch" ;;
  esac
}

# Get latest release version
get_latest_release() {
  local tag version
  tag=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
  if [ -z "$tag" ]; then
    error "Failed to fetch latest version"
  fi
  # Extract bare version from tag (e.g., "@curl-runner/cli@1.16.0" -> "1.16.0")
  version=$(echo "$tag" | sed -E 's/.*@([0-9]+\.[0-9]+\.[0-9]+)$/\1/')
  # Return both: "tag version"
  echo "$tag $version"
}

# Download and verify checksum
download_and_verify() {
  local tag="$1"
  local version="$2"
  local platform="$3"
  local tarball="curl-runner-cli-${version}-${platform}.tar.gz"
  local download_url="https://github.com/${REPO}/releases/download/${tag}/${tarball}"
  local checksum_url="https://github.com/${REPO}/releases/download/${tag}/SHA256SUMS.txt"
  local tmp_dir
  tmp_dir=$(mktemp -d)

  info "Downloading ${tarball}..."
  curl -fsSL "$download_url" -o "${tmp_dir}/${tarball}" || error "Failed to download binary"

  info "Downloading checksums..."
  curl -fsSL "$checksum_url" -o "${tmp_dir}/SHA256SUMS.txt" || error "Failed to download checksums"

  info "Verifying checksum..."
  cd "$tmp_dir"

  # Extract expected checksum for our file
  local expected_checksum
  expected_checksum=$(grep "$tarball" SHA256SUMS.txt | awk '{print $1}')

  if [ -z "$expected_checksum" ]; then
    error "Checksum not found for ${tarball}"
  fi

  # Calculate actual checksum
  local actual_checksum
  if command -v sha256sum &> /dev/null; then
    actual_checksum=$(sha256sum "$tarball" | awk '{print $1}')
  elif command -v shasum &> /dev/null; then
    actual_checksum=$(shasum -a 256 "$tarball" | awk '{print $1}')
  else
    error "No sha256sum or shasum found"
  fi

  if [ "$expected_checksum" != "$actual_checksum" ]; then
    error "Checksum verification failed!\nExpected: ${expected_checksum}\nActual: ${actual_checksum}"
  fi
  success "Checksum verified"

  # Extract binary
  info "Extracting binary..."
  tar -xzf "$tarball"

  # Move to install directory (binary is named curl-runner-{platform})
  mkdir -p "$INSTALL_DIR"
  mv "curl-runner-${platform}" "$INSTALL_DIR/${BINARY_NAME}"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

  # Cleanup
  rm -rf "$tmp_dir"

  success "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
}

# Add to PATH in shell config
add_to_path() {
  local shell_config=""
  local shell_name=""

  # Detect shell
  case "$SHELL" in
    */bash)
      shell_name="bash"
      if [ -f "$HOME/.bashrc" ]; then
        shell_config="$HOME/.bashrc"
      elif [ -f "$HOME/.bash_profile" ]; then
        shell_config="$HOME/.bash_profile"
      fi
      ;;
    */zsh)
      shell_name="zsh"
      shell_config="$HOME/.zshrc"
      ;;
    */fish)
      shell_name="fish"
      shell_config="$HOME/.config/fish/config.fish"
      ;;
    *)
      warn "Unknown shell: $SHELL"
      return
      ;;
  esac

  # Check if already in PATH
  if echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    return
  fi

  # Check if already added to config
  if [ -n "$shell_config" ] && [ -f "$shell_config" ]; then
    if grep -q "CURL_RUNNER_INSTALL" "$shell_config" 2>/dev/null; then
      return
    fi
  fi

  # Add to shell config
  if [ -n "$shell_config" ]; then
    info "Adding ${INSTALL_DIR} to PATH in ${shell_config}..."

    if [ "$shell_name" = "fish" ]; then
      mkdir -p "$(dirname "$shell_config")"
      echo "" >> "$shell_config"
      echo "# curl-runner" >> "$shell_config"
      echo "set -gx CURL_RUNNER_INSTALL \"$INSTALL_DIR\"" >> "$shell_config"
      echo "fish_add_path \$CURL_RUNNER_INSTALL" >> "$shell_config"
    else
      echo "" >> "$shell_config"
      echo "# curl-runner" >> "$shell_config"
      echo "export CURL_RUNNER_INSTALL=\"$INSTALL_DIR\"" >> "$shell_config"
      echo 'export PATH="$CURL_RUNNER_INSTALL:$PATH"' >> "$shell_config"
    fi

    success "Added to PATH in ${shell_config}"
    warn "Restart your shell or run: source ${shell_config}"
  fi
}

main() {
  echo ""
  echo -e "${BLUE}curl-runner installer${NC}"
  echo ""

  local os arch platform tag version release_info

  os=$(detect_os)
  arch=$(detect_arch)
  platform="${os}-${arch}"

  info "Detected platform: ${platform}"

  release_info=$(get_latest_release)
  tag=$(echo "$release_info" | cut -d' ' -f1)
  version=$(echo "$release_info" | cut -d' ' -f2)
  info "Latest version: ${version}"

  download_and_verify "$tag" "$version" "$platform"
  add_to_path

  echo ""
  success "Installation complete!"
  echo ""
  echo "Run 'curl-runner --version' to verify installation"
  echo ""
}

main

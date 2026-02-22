#!/usr/bin/env bash
# ============================================================================
# Darkreach Operator Installer
#
# Installs the darkreach binary, registers an operator account, and sets up
# a background service (systemd on Linux, launchd on macOS).
#
# Usage:
#   curl -sSfL https://install.darkreach.ai | bash
#   ./scripts/install-operator.sh
#   ./scripts/install-operator.sh --uninstall
#   ./scripts/install-operator.sh --no-service
#
# Environment variables:
#   DARKREACH_SERVER    Override default server URL
#   DARKREACH_CHANNEL   Release channel (default: stable)
#   DARKREACH_NO_COLOR  Disable colored output
# ============================================================================

set -euo pipefail

# ── Constants ────────────────────────────────────────────────────

readonly VERSION="1.0.0"
readonly DEFAULT_SERVER="https://api.darkreach.ai"
readonly RELEASE_ENDPOINT="/api/v1/worker/latest"
readonly BINARY_NAME="darkreach"
readonly CONFIG_DIR="$HOME/.darkreach"
readonly SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
readonly LAUNCHD_PLIST_DIR="$HOME/Library/LaunchAgents"
readonly LAUNCHD_LABEL="ai.darkreach.operator"
readonly SYSTEMD_SERVICE="darkreach-operator.service"

# ── Color Output ─────────────────────────────────────────────────

if [[ -z "${DARKREACH_NO_COLOR:-}" ]] && [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    DIM='\033[2m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' BOLD='' DIM='' RESET=''
fi

# ── Logging ──────────────────────────────────────────────────────

info()  { echo -e "${GREEN}[INFO]${RESET}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${RESET}  $*" >&2; }
error() { echo -e "${RED}[ERROR]${RESET} $*" >&2; }
step()  { echo -e "${BLUE}[STEP]${RESET}  ${BOLD}$*${RESET}"; }

die() {
    error "$@"
    exit 1
}

# ── Argument Parsing ─────────────────────────────────────────────

FLAG_UNINSTALL=false
FLAG_NO_SERVICE=false
FLAG_SYSTEM_WIDE=false
FLAG_YES=false
ARG_SERVER="${DARKREACH_SERVER:-$DEFAULT_SERVER}"
ARG_CHANNEL="${DARKREACH_CHANNEL:-stable}"

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --uninstall)     FLAG_UNINSTALL=true ;;
            --no-service)    FLAG_NO_SERVICE=true ;;
            --system)        FLAG_SYSTEM_WIDE=true ;;
            --yes|-y)        FLAG_YES=true ;;
            --server)        ARG_SERVER="$2"; shift ;;
            --server=*)      ARG_SERVER="${1#*=}" ;;
            --channel)       ARG_CHANNEL="$2"; shift ;;
            --channel=*)     ARG_CHANNEL="${1#*=}" ;;
            --help|-h)       usage; exit 0 ;;
            --version|-v)    echo "darkreach-installer $VERSION"; exit 0 ;;
            *)               die "Unknown option: $1. Use --help for usage." ;;
        esac
        shift
    done
}

usage() {
    cat <<'EOF'
Darkreach Operator Installer

USAGE:
    install-operator.sh [OPTIONS]

OPTIONS:
    --uninstall         Remove darkreach binary, config, and services
    --no-service        Install binary only, skip background service setup
    --system            Install to /usr/local/bin (requires sudo)
    --yes, -y           Accept defaults without prompting
    --server <URL>      Coordinator server URL (default: https://api.darkreach.ai)
    --channel <CH>      Release channel: stable, beta (default: stable)
    --help, -h          Show this help
    --version, -v       Show installer version

ENVIRONMENT:
    DARKREACH_SERVER    Override default server URL
    DARKREACH_CHANNEL   Release channel override
    DARKREACH_NO_COLOR  Disable colored output
EOF
}

# ── Platform Detection ───────────────────────────────────────────

detect_os() {
    local uname_s
    uname_s="$(uname -s)"
    case "$uname_s" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*)
            # WSL detection
            if grep -qiE '(microsoft|wsl)' /proc/version 2>/dev/null; then
                echo "linux"
            else
                die "Native Windows is not supported. Please use WSL."
            fi
            ;;
        *)       die "Unsupported operating system: $uname_s" ;;
    esac
}

detect_arch() {
    local uname_m
    uname_m="$(uname -m)"
    case "$uname_m" in
        x86_64|amd64)   echo "x86_64" ;;
        aarch64|arm64)  echo "aarch64" ;;
        *)              die "Unsupported architecture: $uname_m" ;;
    esac
}

# Map our OS/arch names to the artifact field names used by the release API.
# The API uses "os" and "arch" fields (see WorkerReleaseArtifact in operator.rs).
platform_os_field() {
    local os="$1"
    case "$os" in
        linux)  echo "linux" ;;
        macos)  echo "macos" ;;
    esac
}

platform_arch_field() {
    local arch="$1"
    echo "$arch"
}

# ── Dependency Checks ────────────────────────────────────────────

check_dependencies() {
    local missing=()

    if ! command -v curl &>/dev/null; then
        missing+=("curl")
    fi

    # Need at least one checksum tool
    if ! command -v sha256sum &>/dev/null && ! command -v shasum &>/dev/null; then
        missing+=("sha256sum or shasum")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        die "Missing required tools: ${missing[*]}"
    fi
}

# Cross-platform SHA-256 computation
sha256_hash() {
    local file="$1"
    if command -v sha256sum &>/dev/null; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum &>/dev/null; then
        shasum -a 256 "$file" | awk '{print $1}'
    else
        die "No SHA-256 tool available"
    fi
}

# ── Install Path Selection ───────────────────────────────────────

select_install_dir() {
    if [[ "$FLAG_SYSTEM_WIDE" == true ]]; then
        if [[ $EUID -eq 0 ]] || command -v sudo &>/dev/null; then
            echo "/usr/local/bin"
        else
            die "--system requires root or sudo access"
        fi
    else
        # Prefer ~/.local/bin (no sudo needed)
        local user_bin="$HOME/.local/bin"
        mkdir -p "$user_bin"
        echo "$user_bin"
    fi
}

ensure_in_path() {
    local dir="$1"
    if [[ ":$PATH:" != *":$dir:"* ]]; then
        warn "$dir is not in your PATH."
        echo ""
        echo -e "  Add it to your shell profile:"
        local shell_name
        shell_name="$(basename "${SHELL:-/bin/bash}")"
        case "$shell_name" in
            zsh)   echo "    echo 'export PATH=\"$dir:\$PATH\"' >> ~/.zshrc" ;;
            fish)  echo "    fish_add_path $dir" ;;
            *)     echo "    echo 'export PATH=\"$dir:\$PATH\"' >> ~/.bashrc" ;;
        esac
        echo ""
    fi
}

# ── Release Manifest ─────────────────────────────────────────────

fetch_release_manifest() {
    local server="$1"
    local channel="$2"
    local url="${server}${RELEASE_ENDPOINT}?channel=${channel}"

    local tmp_manifest
    tmp_manifest="$(mktemp)"

    info "Fetching release manifest from ${DIM}${url}${RESET}"

    local http_code
    http_code=$(curl -sSfL -w '%{http_code}' -o "$tmp_manifest" "$url" 2>/dev/null) || {
        rm -f "$tmp_manifest"
        die "Failed to fetch release manifest from $url (is the server reachable?)"
    }

    if [[ "$http_code" -ne 200 ]]; then
        rm -f "$tmp_manifest"
        die "Release manifest returned HTTP $http_code"
    fi

    cat "$tmp_manifest"
    rm -f "$tmp_manifest"
}

# Extract a field from JSON without jq (portable).
# Usage: json_field '{"version":"1.0"}' "version" -> 1.0
json_field() {
    local json="$1"
    local field="$2"
    echo "$json" | sed -n 's/.*"'"$field"'"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
}

# Extract artifact info for a given OS/arch from the manifest JSON.
# Returns "url sha256" or empty string if not found.
find_artifact() {
    local manifest="$1"
    local target_os="$2"
    local target_arch="$3"

    # Parse artifacts array - look for matching os and arch fields.
    # The manifest JSON has artifacts with "os", "arch", "url", "sha256" fields.
    # We use a simple line-by-line approach that works without jq.
    local in_artifact=false
    local found_os=false
    local found_arch=false
    local art_url=""
    local art_sha=""

    while IFS= read -r line; do
        # Detect artifact object boundaries
        if echo "$line" | grep -q '{'; then
            in_artifact=true
            found_os=false
            found_arch=false
            art_url=""
            art_sha=""
        fi

        if [[ "$in_artifact" == true ]]; then
            # Check os field
            if echo "$line" | grep -q '"os"'; then
                local val
                val=$(echo "$line" | sed -n 's/.*"os"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
                if [[ "$val" == "$target_os" ]]; then
                    found_os=true
                fi
            fi

            # Check arch field
            if echo "$line" | grep -q '"arch"'; then
                local val
                val=$(echo "$line" | sed -n 's/.*"arch"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
                if [[ "$val" == "$target_arch" ]]; then
                    found_arch=true
                fi
            fi

            # Extract url
            if echo "$line" | grep -q '"url"'; then
                art_url=$(echo "$line" | sed -n 's/.*"url"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
            fi

            # Extract sha256
            if echo "$line" | grep -q '"sha256"'; then
                art_sha=$(echo "$line" | sed -n 's/.*"sha256"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
            fi
        fi

        # End of artifact object
        if echo "$line" | grep -q '}'; then
            if [[ "$found_os" == true ]] && [[ "$found_arch" == true ]] && [[ -n "$art_url" ]] && [[ -n "$art_sha" ]]; then
                echo "$art_url $art_sha"
                return 0
            fi
            in_artifact=false
        fi
    done <<< "$manifest"

    return 1
}

# ── Binary Download & Verification ───────────────────────────────

download_binary() {
    local url="$1"
    local expected_sha="$2"
    local dest="$3"

    local tmp_archive
    tmp_archive="$(mktemp)"

    info "Downloading binary from ${DIM}${url}${RESET}"

    curl -sSfL -o "$tmp_archive" "$url" || {
        rm -f "$tmp_archive"
        die "Download failed from $url"
    }

    step "Verifying SHA-256 checksum"
    local actual_sha
    actual_sha=$(sha256_hash "$tmp_archive")

    # Case-insensitive comparison
    if [[ "${actual_sha,,}" != "${expected_sha,,}" ]]; then
        rm -f "$tmp_archive"
        die "Checksum mismatch!\n  Expected: $expected_sha\n  Got:      $actual_sha\n\nThe download may be corrupted or tampered with. Aborting."
    fi

    info "Checksum verified: ${DIM}${actual_sha:0:16}...${RESET}"

    # Determine if the download is a tar.gz archive or a raw binary
    local file_type
    file_type=$(file -b "$tmp_archive" 2>/dev/null || echo "unknown")

    if echo "$file_type" | grep -qiE '(gzip|tar)'; then
        step "Extracting archive"
        local tmp_extract
        tmp_extract="$(mktemp -d)"
        tar -xzf "$tmp_archive" -C "$tmp_extract" 2>/dev/null || {
            rm -f "$tmp_archive"
            rm -rf "$tmp_extract"
            die "Failed to extract archive"
        }

        # Find the darkreach binary in the extracted contents
        local found_binary
        found_binary=$(find "$tmp_extract" -name "$BINARY_NAME" -type f 2>/dev/null | head -1)

        if [[ -z "$found_binary" ]]; then
            rm -f "$tmp_archive"
            rm -rf "$tmp_extract"
            die "No '$BINARY_NAME' binary found in archive"
        fi

        cp "$found_binary" "$dest"
        rm -rf "$tmp_extract"
    else
        # Raw binary
        cp "$tmp_archive" "$dest"
    fi

    rm -f "$tmp_archive"
    chmod +x "$dest"
}

install_binary() {
    local source="$1"
    local install_dir="$2"
    local target="$install_dir/$BINARY_NAME"

    step "Installing binary to $target"

    if [[ "$install_dir" == "/usr/local/bin" ]] && [[ $EUID -ne 0 ]]; then
        sudo cp "$source" "$target"
        sudo chmod +x "$target"
    else
        mkdir -p "$install_dir"
        cp "$source" "$target"
        chmod +x "$target"
    fi

    info "Binary installed to $target"
}

# ── Registration ─────────────────────────────────────────────────

prompt_registration() {
    local server="$1"
    local binary="$2"

    # Skip if already registered
    if [[ -f "$CONFIG_DIR/config.toml" ]]; then
        info "Already registered (config found at $CONFIG_DIR/config.toml)"
        return 0
    fi

    echo ""
    echo -e "${BOLD}Operator Registration${RESET}"
    echo -e "${DIM}Register to receive an API key and start contributing compute.${RESET}"
    echo ""

    local reg_server="$server"
    local reg_username=""
    local reg_email=""

    if [[ "$FLAG_YES" == true ]]; then
        warn "Cannot skip registration prompts with --yes (username and email required)"
    fi

    # Server URL
    read -rp "  Server URL [$reg_server]: " input_server
    if [[ -n "$input_server" ]]; then
        reg_server="$input_server"
    fi

    # Username
    while [[ -z "$reg_username" ]]; do
        read -rp "  Username: " reg_username
        if [[ -z "$reg_username" ]]; then
            warn "Username is required"
        fi
    done

    # Email
    while [[ -z "$reg_email" ]]; do
        read -rp "  Email: " reg_email
        if [[ -z "$reg_email" ]]; then
            warn "Email is required"
        fi
    done

    echo ""
    step "Registering with $reg_server"

    "$binary" register \
        --server="$reg_server" \
        --username="$reg_username" \
        --email="$reg_email" || {
        warn "Registration failed. You can register later with:"
        echo "  $binary register --server=$reg_server --username=<username> --email=<email>"
        return 1
    }

    info "Registration successful"
}

# ── Service Installation ─────────────────────────────────────────

install_systemd_service() {
    local binary_path="$1"

    step "Installing systemd user service"

    mkdir -p "$SYSTEMD_USER_DIR"

    cat > "$SYSTEMD_USER_DIR/$SYSTEMD_SERVICE" <<UNIT
[Unit]
Description=Darkreach Operator Node
Documentation=https://darkreach.ai/docs/operator
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${binary_path} run
Restart=on-failure
RestartSec=30
Environment=DARKREACH_AUTO_UPDATE=1
Environment=RUST_LOG=darkreach=info

# Resource limits
LimitNOFILE=65536

# Graceful shutdown
KillSignal=SIGTERM
TimeoutStopSec=60

[Install]
WantedBy=default.target
UNIT

    # Enable lingering so the user service runs without an active login session
    if command -v loginctl &>/dev/null; then
        loginctl enable-linger "$(whoami)" 2>/dev/null || true
    fi

    systemctl --user daemon-reload 2>/dev/null || true
    systemctl --user enable "$SYSTEMD_SERVICE" 2>/dev/null || {
        warn "Could not enable systemd service. You may need to start it manually."
        return 1
    }

    info "Systemd user service installed: $SYSTEMD_USER_DIR/$SYSTEMD_SERVICE"
}

install_launchd_plist() {
    local binary_path="$1"

    step "Installing launchd agent"

    mkdir -p "$LAUNCHD_PLIST_DIR"

    local plist_path="$LAUNCHD_PLIST_DIR/${LAUNCHD_LABEL}.plist"

    cat > "$plist_path" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${LAUNCHD_LABEL}</string>

    <key>ProgramArguments</key>
    <array>
        <string>${binary_path}</string>
        <string>run</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>NetworkState</key>
        <true/>
    </dict>

    <key>ThrottleInterval</key>
    <integer>30</integer>

    <key>EnvironmentVariables</key>
    <dict>
        <key>DARKREACH_AUTO_UPDATE</key>
        <string>1</string>
        <key>RUST_LOG</key>
        <string>darkreach=info</string>
        <key>HOME</key>
        <string>${HOME}</string>
        <key>PATH</key>
        <string>${binary_path%/*}:/usr/local/bin:/usr/bin:/bin</string>
    </dict>

    <key>StandardOutPath</key>
    <string>${CONFIG_DIR}/operator.log</string>

    <key>StandardErrorPath</key>
    <string>${CONFIG_DIR}/operator.log</string>

    <key>ProcessType</key>
    <string>Background</string>
</dict>
</plist>
PLIST

    info "Launchd plist installed: $plist_path"
}

start_service() {
    local os="$1"

    step "Starting operator service"

    case "$os" in
        linux)
            systemctl --user start "$SYSTEMD_SERVICE" 2>/dev/null || {
                warn "Could not start service. Start manually with:"
                echo "  systemctl --user start $SYSTEMD_SERVICE"
                return 1
            }
            info "Service started"
            ;;
        macos)
            local plist_path="$LAUNCHD_PLIST_DIR/${LAUNCHD_LABEL}.plist"
            # Unload first to avoid "already loaded" error
            launchctl bootout "gui/$(id -u)/${LAUNCHD_LABEL}" 2>/dev/null || true
            launchctl bootstrap "gui/$(id -u)" "$plist_path" 2>/dev/null || {
                # Fall back to legacy load command
                launchctl load -w "$plist_path" 2>/dev/null || {
                    warn "Could not start service. Start manually with:"
                    echo "  launchctl bootstrap gui/$(id -u) $plist_path"
                    return 1
                }
            }
            info "Service started"
            ;;
    esac
}

# ── Uninstall ────────────────────────────────────────────────────

do_uninstall() {
    local os
    os="$(detect_os)"

    echo -e "${BOLD}Darkreach Operator Uninstaller${RESET}"
    echo ""

    if [[ "$FLAG_YES" != true ]]; then
        read -rp "This will remove the darkreach binary, config, and services. Continue? [y/N] " confirm
        if [[ "$confirm" != [yY] ]]; then
            info "Uninstall cancelled"
            exit 0
        fi
    fi

    # Stop and remove services
    case "$os" in
        linux)
            if [[ -f "$SYSTEMD_USER_DIR/$SYSTEMD_SERVICE" ]]; then
                step "Stopping systemd service"
                systemctl --user stop "$SYSTEMD_SERVICE" 2>/dev/null || true
                systemctl --user disable "$SYSTEMD_SERVICE" 2>/dev/null || true
                rm -f "$SYSTEMD_USER_DIR/$SYSTEMD_SERVICE"
                systemctl --user daemon-reload 2>/dev/null || true
                info "Systemd service removed"
            fi
            ;;
        macos)
            local plist_path="$LAUNCHD_PLIST_DIR/${LAUNCHD_LABEL}.plist"
            if [[ -f "$plist_path" ]]; then
                step "Stopping launchd service"
                launchctl bootout "gui/$(id -u)/${LAUNCHD_LABEL}" 2>/dev/null || \
                    launchctl unload "$plist_path" 2>/dev/null || true
                rm -f "$plist_path"
                info "Launchd service removed"
            fi
            ;;
    esac

    # Remove binary
    for bin_path in "$HOME/.local/bin/$BINARY_NAME" "/usr/local/bin/$BINARY_NAME"; do
        if [[ -f "$bin_path" ]]; then
            step "Removing binary: $bin_path"
            if [[ "$bin_path" == "/usr/local/bin/"* ]] && [[ $EUID -ne 0 ]]; then
                sudo rm -f "$bin_path"
            else
                rm -f "$bin_path"
            fi
            info "Binary removed"
        fi
    done

    # Remove config directory
    if [[ -d "$CONFIG_DIR" ]]; then
        step "Removing config directory: $CONFIG_DIR"
        if [[ "$FLAG_YES" != true ]]; then
            read -rp "Remove config including API key? [y/N] " confirm_config
            if [[ "$confirm_config" == [yY] ]]; then
                rm -rf "$CONFIG_DIR"
                info "Config removed"
            else
                info "Config preserved at $CONFIG_DIR"
            fi
        else
            rm -rf "$CONFIG_DIR"
            info "Config removed"
        fi
    fi

    echo ""
    info "Darkreach operator uninstalled successfully"
    exit 0
}

# ── Print Success ────────────────────────────────────────────────

print_success() {
    local os="$1"
    local binary_path="$2"

    echo ""
    echo -e "${GREEN}${BOLD}Darkreach operator installed successfully!${RESET}"
    echo ""
    echo -e "${BOLD}Useful commands:${RESET}"
    echo ""
    echo -e "  ${DIM}# Check operator status${RESET}"

    case "$os" in
        linux)
            echo "  systemctl --user status $SYSTEMD_SERVICE"
            echo ""
            echo -e "  ${DIM}# View logs${RESET}"
            echo "  journalctl --user -u $SYSTEMD_SERVICE -f"
            echo ""
            echo -e "  ${DIM}# Stop the operator${RESET}"
            echo "  systemctl --user stop $SYSTEMD_SERVICE"
            echo ""
            echo -e "  ${DIM}# Restart the operator${RESET}"
            echo "  systemctl --user restart $SYSTEMD_SERVICE"
            ;;
        macos)
            echo "  launchctl print gui/$(id -u)/${LAUNCHD_LABEL}"
            echo ""
            echo -e "  ${DIM}# View logs${RESET}"
            echo "  tail -f $CONFIG_DIR/operator.log"
            echo ""
            echo -e "  ${DIM}# Stop the operator${RESET}"
            echo "  launchctl bootout gui/$(id -u)/${LAUNCHD_LABEL}"
            echo ""
            echo -e "  ${DIM}# Start the operator${RESET}"
            echo "  launchctl bootstrap gui/$(id -u) $LAUNCHD_PLIST_DIR/${LAUNCHD_LABEL}.plist"
            ;;
    esac

    echo ""
    echo -e "  ${DIM}# Uninstall${RESET}"
    echo "  $binary_path --uninstall  ${DIM}# or re-run this script with --uninstall${RESET}"
    echo ""
    echo -e "  ${DIM}# Dashboard${RESET}"
    echo "  https://app.darkreach.ai"
    echo ""
}

# ── Main ─────────────────────────────────────────────────────────

main() {
    parse_args "$@"

    echo ""
    echo -e "${BOLD}Darkreach Operator Installer${RESET} ${DIM}v${VERSION}${RESET}"
    echo ""

    # Uninstall mode
    if [[ "$FLAG_UNINSTALL" == true ]]; then
        do_uninstall
    fi

    # Pre-flight checks
    check_dependencies

    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"

    info "Detected platform: ${BOLD}${os}/${arch}${RESET}"

    # Fetch release manifest
    step "Checking latest release (channel: $ARG_CHANNEL)"
    local manifest
    manifest="$(fetch_release_manifest "$ARG_SERVER" "$ARG_CHANNEL")"

    local release_version
    release_version="$(json_field "$manifest" "version")"
    if [[ -z "$release_version" ]]; then
        die "Could not parse version from release manifest"
    fi
    info "Latest version: ${BOLD}$release_version${RESET}"

    # Find matching artifact
    local api_os api_arch
    api_os="$(platform_os_field "$os")"
    api_arch="$(platform_arch_field "$arch")"

    local artifact_info
    artifact_info="$(find_artifact "$manifest" "$api_os" "$api_arch")" || {
        die "No release artifact found for ${os}/${arch} in channel '$ARG_CHANNEL'"
    }

    local artifact_url artifact_sha
    artifact_url="$(echo "$artifact_info" | awk '{print $1}')"
    artifact_sha="$(echo "$artifact_info" | awk '{print $2}')"

    if [[ -z "$artifact_url" ]] || [[ -z "$artifact_sha" ]]; then
        die "Failed to parse artifact URL or checksum from manifest"
    fi

    # Select install directory
    local install_dir
    install_dir="$(select_install_dir)"
    local binary_path="$install_dir/$BINARY_NAME"

    # Check for existing installation
    if [[ -f "$binary_path" ]]; then
        local current_version
        current_version=$("$binary_path" --version 2>/dev/null | awk '{print $NF}' || echo "unknown")
        if [[ "$current_version" == "$release_version" ]]; then
            info "Already at version $release_version"
            if [[ "$FLAG_YES" != true ]]; then
                read -rp "  Reinstall anyway? [y/N] " confirm
                if [[ "$confirm" != [yY] ]]; then
                    info "Skipping binary download"
                    # Still offer service setup if not installed
                    if [[ "$FLAG_NO_SERVICE" != true ]]; then
                        install_service "$os" "$binary_path"
                    fi
                    print_success "$os" "$binary_path"
                    exit 0
                fi
            fi
        else
            info "Upgrading from $current_version to $release_version"
        fi
    fi

    # Download and verify
    local tmp_binary
    tmp_binary="$(mktemp)"

    step "Downloading darkreach $release_version"
    download_binary "$artifact_url" "$artifact_sha" "$tmp_binary"

    # Install
    install_binary "$tmp_binary" "$install_dir"
    rm -f "$tmp_binary"

    # Verify installation
    if ! "$binary_path" --version &>/dev/null; then
        die "Installation verification failed: $binary_path is not executable"
    fi

    local installed_version
    installed_version=$("$binary_path" --version 2>/dev/null | awk '{print $NF}' || echo "unknown")
    info "Installed darkreach $installed_version"

    # Ensure install dir is in PATH
    ensure_in_path "$install_dir"

    # Create config directory
    mkdir -p "$CONFIG_DIR"

    # Registration
    if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
        prompt_registration "$ARG_SERVER" "$binary_path" || true
    else
        info "Operator already registered (config found at $CONFIG_DIR/config.toml)"
    fi

    # Service installation
    if [[ "$FLAG_NO_SERVICE" != true ]]; then
        install_service "$os" "$binary_path"
    else
        info "Skipping service installation (--no-service)"
    fi

    print_success "$os" "$binary_path"
}

install_service() {
    local os="$1"
    local binary_path="$2"

    case "$os" in
        linux)
            install_systemd_service "$binary_path"
            start_service "$os"
            ;;
        macos)
            install_launchd_plist "$binary_path"
            start_service "$os"
            ;;
    esac
}

# ── Entry Point ──────────────────────────────────────────────────

main "$@"

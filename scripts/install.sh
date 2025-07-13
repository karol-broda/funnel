#!/bin/bash
#
# funnel installation script
#
# this script downloads and installs the latest or a specific version of the funnel
# client, verifying the download with a checksum.
#
# usage:
#   curl -sfL https://raw.githubusercontent.com/karol-broda/funnel/master/scripts/install.sh | bash
#   curl -sfL https://raw.githubusercontent.com/karol-broda/funnel/master/scripts/install.sh | bash -s -- -v v0.0.5
#
# options:
#   -v, --version VERSION    install a specific version (e.g., v0.0.5)
#   -l, --list-versions      list available versions
#   -h, --help               show this help message

set -euo pipefail

readonly REPO="karol-broda/funnel"
readonly PROJECT_NAME="funnel"
readonly CLIENT_NAME="funnel"

readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # no color

TEMP_BINARY_PATH=""
TMP_DIR=""
INSTALL_DIR=""

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

die() {
    log_error "$1"
    exit 1
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

show_usage() {
    echo "funnel installation script"
    echo
    echo "usage: $0 [options]"
    echo
    echo "options:"
    echo "  -v, --version VERSION    install specific version (e.g., -v v0.0.2)"
    echo "  -b, --bin-dir DIR        install to a specific directory (defaults to ~/.local/bin)"
    echo "  --global                 install to /usr/local/bin (requires sudo)"
    echo "  -l, --list-versions      list available versions"
    echo "  -h, --help               show this help message"
}

list_versions() {
    log_info "fetching available versions from ${REPO}..."
    local releases_url="https://api.github.com/repos/${REPO}/releases"
    local response
    if ! response=$(curl -sL "${releases_url}"); then
        die "failed to fetch releases from github api"
    fi

    echo "available versions:"
    if command_exists jq; then
        echo "${response}" | jq -r '.[].tag_name' | head -20 | sed 's/^/  /'
    else
        echo "${response}" | grep -o '"tag_name": *"[^"]*"' | cut -d'"' -f4 | head -20 | sed 's/^/  /'
    fi
    echo
    log_info "use -v <version> to install a specific version."
}

init() {
    SPECIFIC_VERSION=""
    INSTALL_DIR=""
    USE_GLOBAL=false
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--version)
                SPECIFIC_VERSION="$2"
                shift 2
                ;;
            -b|--bin-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --global)
                USE_GLOBAL=true
                shift 1
                ;;
            -l|--list-versions)
                list_versions
                exit 0
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                die "unknown option: $1"
                ;;
        esac
    done

    if [[ "${USE_GLOBAL}" == true ]]; then
        INSTALL_DIR="/usr/local/bin"
    elif [[ -z "${INSTALL_DIR}" ]]; then
        INSTALL_DIR="${HOME}/.local/bin"
    fi

    command_exists curl || die "curl is not installed. please install it to continue."
    command_exists tar || die "tar is not installed. please install it to continue."
}

detect_platform() {
    local os
    local arch
    case "$(uname -s)" in
        Darwin) os="darwin" ;;
        Linux) os="linux" ;;
        *) die "unsupported operating system: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="amd64" ;;
        arm64|aarch64) arch="arm64" ;;
        *) die "unsupported architecture: $(uname -m)" ;;
    esac

    DETECTED_OS="$os"
    DETECTED_ARCH="$arch"
    log_info "detected platform: ${DETECTED_OS}/${DETECTED_ARCH}"
}

fetch_release_info() {
    local api_url
    if [[ -n "${SPECIFIC_VERSION}" ]]; then
        log_info "fetching release info for version: ${SPECIFIC_VERSION}"
        api_url="https://api.github.com/repos/${REPO}/releases/tags/${SPECIFIC_VERSION}"
    else
        log_info "fetching latest release info from github..."
        api_url="https://api.github.com/repos/${REPO}/releases/latest"
    fi

    local response
    if ! response=$(curl -sL "${api_url}"); then
        die "failed to fetch release information."
    fi

    if echo "${response}" | grep -q '"message": "Not Found"'; then
        die "release not found. please check that version '${SPECIFIC_VERSION:-latest}' exists."
    fi

    RELEASE_TAG=$(echo "${response}" | grep -o '"tag_name": *"[^"]*"' | cut -d'"' -f4)
    [[ -n "${RELEASE_TAG}" ]] || die "could not determine release tag."

    log_info "using release: ${RELEASE_TAG}"
}

download_and_verify() {
    local version_without_v="${RELEASE_TAG#v}"
    local archive_name="${PROJECT_NAME}_${CLIENT_NAME}_${version_without_v}_${DETECTED_OS}_${DETECTED_ARCH}.tar.gz"
    local checksum_file="${PROJECT_NAME}_${version_without_v}_checksums.txt"

    local download_url="https://github.com/${REPO}/releases/download/${RELEASE_TAG}/${archive_name}"
    local checksum_url="https://github.com/${REPO}/releases/download/${RELEASE_TAG}/${checksum_file}"

    TMP_DIR=$(mktemp -d)
    if [ $? -ne 0 ]; then
        die "failed to create temporary directory."
    fi

    # shellcheck disable=SC2064
    trap 'rm -rf -- "$TMP_DIR"' EXIT

    log_info "downloading archive from ${download_url}"
    if ! curl -sSLf -o "${TMP_DIR}/${archive_name}" "${download_url}"; then
        die "failed to download archive: ${archive_name}"
    fi

    log_info "downloading checksums from ${checksum_url}"
    if ! curl -sSLf -o "${TMP_DIR}/checksums.txt" "${checksum_url}"; then
        die "failed to download checksums."
    fi
    
    log_info "verifying checksum..."
    (
        cd "${TMP_DIR}"
        local checksum_line
        checksum_line=$(grep "${archive_name}" checksums.txt)
        
        if [[ -z "${checksum_line}" ]]; then
            die "could not find checksum for ${archive_name} in checksums.txt"
        fi

        if command_exists sha256sum; then
            echo "${checksum_line}" > checksums.txt.tmp
            if ! sha256sum -c checksums.txt.tmp >/dev/null 2>&1; then
                rm checksums.txt.tmp
                die "checksum verification failed for ${archive_name}."
            fi
            rm checksums.txt.tmp
        elif command_exists shasum; then
            if ! echo "${checksum_line}" | shasum -a 256 -c --status; then
                 die "checksum verification failed for ${archive_name}."
            fi
        else
            die "could not find 'sha256sum' or 'shasum' to verify checksums."
        fi
        
        log_success "checksum verified successfully."
    )

    log_info "extracting binary from archive..."
    if ! tar -xzf "${TMP_DIR}/${archive_name}" -C "${TMP_DIR}" "${CLIENT_NAME}"; then
        die "failed to extract binary from archive."
    fi
    
    TEMP_BINARY_PATH="${TMP_DIR}/${CLIENT_NAME}"
    log_success "binary extracted to temporary location: ${TEMP_BINARY_PATH}"
}

install_binary() {
    log_info "installing '${CLIENT_NAME}' to '${INSTALL_DIR}'..."

    local use_sudo=""
    if [[ ! -w "${INSTALL_DIR}" ]]; then
        if command_exists sudo; then
            use_sudo="sudo"
            log_warning "write permission to ${INSTALL_DIR} is required. using sudo."
        else
            die "write permission to ${INSTALL_DIR} is required, and sudo is not available."
        fi
    fi

    if [[ ! -d "${INSTALL_DIR}" ]]; then
        log_info "creating directory ${INSTALL_DIR}"
        if ! ${use_sudo} mkdir -p "${INSTALL_DIR}"; then
            die "failed to create installation directory."
        fi
    fi

    local install_path="${INSTALL_DIR}/${CLIENT_NAME}"

    if ! ${use_sudo} mv "${TEMP_BINARY_PATH}" "${install_path}"; then
        die "failed to move binary to ${INSTALL_DIR}"
    fi

    # on macos, remove the quarantine attribute
    if [[ "$(uname -s)" == "Darwin" ]]; then
        if xattr "${install_path}" 2>/dev/null | grep -q "com.apple.quarantine"; then
            log_info "removing quarantine attribute on macos..."
            if ! ${use_sudo} xattr -d com.apple.quarantine "${install_path}"; then
                log_warning "failed to remove quarantine attribute. you might need to approve the binary manually."
            fi
        fi
    fi

    log_success "successfully installed '${CLIENT_NAME}' to '${install_path}'"
}

post_install() {
    verify_installation
    check_path
}

verify_installation() {
    log_info "verifying installation..."
    local installed_path="${INSTALL_DIR}/${CLIENT_NAME}"

    if [[ ! -x "${installed_path}" ]]; then
        log_warning "binary not found at ${installed_path} or not executable."
        return
    fi

    if ! "${installed_path}" version >/dev/null 2>&1; then
        log_warning "could not run '${CLIENT_NAME} version'. please verify the installation manually."
        return
    fi

    local version
    version=$("${installed_path}" version)
    log_success "installation verified successfully. version details:\n${version}"
}

check_path() {
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*)
            ;;
        *)
            log_warning "your shell may not be able to find ${CLIENT_NAME}."
            echo
            echo "please add the following line to your shell profile (e.g., ~/.zshrc or ~/.bashrc):"
            echo "  export PATH=\"${INSTALL_DIR}:\\$PATH\""
            echo
            echo "then, restart your shell or run 'source <your_profile_file>' to apply the changes."
            ;;
    esac
}

main() {
    echo "--- funnel installation script ---"
    init "$@"
    detect_platform
    fetch_release_info
    download_and_verify
    install_binary
    post_install
}

main "$@" 
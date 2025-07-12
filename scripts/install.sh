#!/bin/bash

set -e

REPO="karol-broda/funnel"
CLIENT_NAME="funnel"
INSTALL_DIR="/usr/local/bin"
GITHUB_API_URL="https://api.github.com/repos/${REPO}/releases/latest"
SPECIFIC_VERSION=""

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_usage() {
    echo "funnel installation script"
    echo
    echo "usage: $0 [options]"
    echo
    echo "options:"
    echo "  -v, --version VERSION    install specific version (e.g., -v v0.0.2)"
    echo "  -l, --list-versions      list available versions"
    echo "  -h, --help              show this help message"
    echo
    echo "examples:"
    echo "  $0                      # install latest version"
    echo "  $0 -v v0.0.2           # install version v0.0.2"
    echo "  $0 --version v0.0.1     # install version v0.0.1"
    echo "  $0 --list-versions      # show available versions"
    echo
}

list_versions() {
    log_info "fetching available versions..."
    
    local response
    response=$(curl -s "https://api.github.com/repos/${REPO}/releases")
    
    if [[ $? -ne 0 ]]; then
        log_error "failed to fetch releases from GitHub API"
        exit 1
    fi
    
    echo "available versions:"
    
    if command -v jq >/dev/null 2>&1; then
        echo "${response}" | jq -r '.[].tag_name // empty' 2>/dev/null | head -20 | sed 's/^/  /'
    else
        echo "${response}" | grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4 | head -20 | sed 's/^/  /'
    fi
    
    echo
    log_info "use -v <version> to install a specific version"
}

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--version)
                if [[ -n $2 && $2 != -* ]]; then
                    SPECIFIC_VERSION="$2"
                    shift 2
                else
                    log_error "version argument requires a value"
                    show_usage
                    exit 1
                fi
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
                log_error "unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    if [[ -n "${SPECIFIC_VERSION}" ]]; then
        GITHUB_API_URL="https://api.github.com/repos/${REPO}/releases/tags/${SPECIFIC_VERSION}"
        log_info "requesting specific version: ${SPECIFIC_VERSION}"
    fi
}

detect_platform() {
    local os=""
    local arch=""
    
    case "$(uname -s)" in
        Darwin*)
            os="darwin"
            ;;
        Linux*)
            os="linux"
            ;;
        *)
            log_error "unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64)
            arch="amd64"
            ;;
        arm64|aarch64)
            arch="arm64"
            ;;
        *)
            log_error "unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    
    PLATFORM="${os}/${arch}"
    DETECTED_OS="${os}"
    DETECTED_ARCH="${arch}"
    
    log_info "detected platform: ${PLATFORM}"
}

check_dependencies() {
    local missing_deps=()
    
    if ! command -v curl >/dev/null 2>&1; then
        missing_deps+=("curl")
    fi
    
    if ! command -v jq >/dev/null 2>&1; then
        log_warning "jq not found. will use basic json parsing (less reliable)."
    fi
    
    if ! command -v tar >/dev/null 2>&1; then
        missing_deps+=("tar")
    fi
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "missing required dependencies: ${missing_deps[*]}"
        log_error "please install the missing tools and try again"
        exit 1
    fi
}

safe_json_extract() {
    local json_data="$1"
    local field="$2"
    local result=""
    
    if command -v jq >/dev/null 2>&1; then
        result=$(echo "${json_data}" | jq -r ".${field} // empty" 2>/dev/null || echo "")
        
        if [[ -z "${result}" ]] || [[ "${result}" == "null" ]]; then
            result=$(echo "${json_data}" | grep -o "\"${field}\":\"[^\"]*\"" | cut -d'"' -f4 | head -1)
        fi
    else
        # fallback to grep-based extraction
        result=$(echo "${json_data}" | grep -o "\"${field}\":\"[^\"]*\"" | cut -d'"' -f4 | head -1)
    fi
    
    echo "${result}"
}

safe_extract_download_url() {
    local json_data="$1"
    local archive_name="$2"
    local result=""
    
    if command -v jq >/dev/null 2>&1; then
        result=$(echo "${json_data}" | jq -r ".assets[]? | select(.name == \"${archive_name}\") | .browser_download_url // empty" 2>/dev/null || echo "")
        
        if [[ -z "${result}" ]] || [[ "${result}" == "null" ]]; then
            result=$(echo "${json_data}" | grep -o "\"browser_download_url\":\"[^\"]*${archive_name}\"" | cut -d'"' -f4)
        fi
    else
        result=$(echo "${json_data}" | grep -o "\"browser_download_url\":\"[^\"]*${archive_name}\"" | cut -d'"' -f4)
    fi
    
    echo "${result}"
}

get_latest_release() {
    if [[ -n "${SPECIFIC_VERSION}" ]]; then
        log_info "fetching release information for version ${SPECIFIC_VERSION}..."
    else
        log_info "fetching latest release information from GitHub..."
    fi
    
    local response
    response=$(curl -s "${GITHUB_API_URL}")
    
    if [[ $? -ne 0 ]]; then
        log_error "failed to fetch release information from GitHub API"
        exit 1
    fi
    
    if echo "${response}" | grep -q '"message": "Not Found"'; then
        log_error "version ${SPECIFIC_VERSION} not found"
        log_error "please check that the version exists in the repository"
        exit 1
    fi
    
    RELEASE_TAG=$(safe_json_extract "${response}" "tag_name")
    RELEASE_NAME=$(safe_json_extract "${response}" "name")
    
    if [[ -z "${RELEASE_TAG}" ]] || [[ "${RELEASE_TAG}" == "null" ]]; then
        log_error "could not determine release tag"
        if [[ -n "${SPECIFIC_VERSION}" ]]; then
            log_error "version ${SPECIFIC_VERSION} may not exist or may not have releases"
        fi
        exit 1
    fi
    
    if [[ -n "${SPECIFIC_VERSION}" ]] && [[ "${RELEASE_TAG}" != "${SPECIFIC_VERSION}" ]]; then
        log_error "requested version ${SPECIFIC_VERSION} but got ${RELEASE_TAG}"
        exit 1
    fi
    
    ARCHIVE_NAME="${CLIENT_NAME}-${RELEASE_TAG}-${DETECTED_OS}-${DETECTED_ARCH}.tar.gz"
    ARCHIVE_EXT="tar.gz"
    
    DOWNLOAD_URL=$(safe_extract_download_url "${response}" "${ARCHIVE_NAME}")
    
    if [[ -z "${DOWNLOAD_URL}" ]] || [[ "${DOWNLOAD_URL}" == "null" ]]; then
        log_error "could not find download URL for ${ARCHIVE_NAME}"
        log_error "available releases might not include binaries for your platform (${PLATFORM})"
        
        if command -v jq >/dev/null 2>&1; then
            local available_assets
            available_assets=$(echo "${response}" | jq -r '.assets[]?.name // empty' 2>/dev/null || echo "none")
            if [[ "${available_assets}" != "none" ]] && [[ -n "${available_assets}" ]]; then
                log_error "available assets for this release:"
                echo "${available_assets}" | sed 's/^/  /'
            fi
        fi
        
        exit 1
    fi
    
    if [[ -n "${SPECIFIC_VERSION}" ]]; then
        log_info "selected version: ${RELEASE_NAME} (${RELEASE_TAG})"
    else
        log_info "latest release: ${RELEASE_NAME} (${RELEASE_TAG})"
    fi
    log_info "archive: ${ARCHIVE_NAME}"
    log_info "download URL: ${DOWNLOAD_URL}"
}

download_binary() {
    local temp_dir
    temp_dir=$(mktemp -d)
    
    log_info "downloading ${CLIENT_NAME} archive..."
    
    if ! curl -L -o "${temp_dir}/${ARCHIVE_NAME}" "${DOWNLOAD_URL}"; then
        log_error "failed to download archive"
        rm -rf "${temp_dir}"
        exit 1
    fi
    
    if [[ ! -f "${temp_dir}/${ARCHIVE_NAME}" ]]; then
        log_error "downloaded file not found"
        rm -rf "${temp_dir}"
        exit 1
    fi
    
    local file_size
    file_size=$(wc -c < "${temp_dir}/${ARCHIVE_NAME}")
    if [[ "${file_size}" -eq 0 ]]; then
        log_error "downloaded file is empty"
        rm -rf "${temp_dir}"
        exit 1
    fi
    
    log_success "downloaded ${CLIENT_NAME} archive (${file_size} bytes)"
    
    log_info "extracting archive..."
    
    if ! tar -xzf "${temp_dir}/${ARCHIVE_NAME}" -C "${temp_dir}"; then
        log_error "failed to extract tar.gz archive"
        rm -rf "${temp_dir}"
        exit 1
    fi
    
    local found_binary=""
    found_binary=$(find "${temp_dir}" -name "${CLIENT_NAME}" -type f | head -1)
    
    if [[ -z "${found_binary}" ]]; then
        log_error "could not find ${CLIENT_NAME} in extracted archive"
        log_error "extracted files:"
        find "${temp_dir}" -type f | sed 's/^/  /'
        rm -rf "${temp_dir}"
        exit 1
    fi
    
    log_success "extracted binary: ${found_binary}"
    TEMP_BINARY_PATH="${found_binary}"
}

install_binary() {
    log_info "installing ${CLIENT_NAME} to ${INSTALL_DIR}..."
    
    if [[ ! -d "${INSTALL_DIR}" ]]; then
        log_info "creating install directory: ${INSTALL_DIR}"
        if ! sudo mkdir -p "${INSTALL_DIR}"; then
            log_error "failed to create install directory"
            exit 1
        fi
    fi
    
    local use_sudo=""
    if [[ ! -w "${INSTALL_DIR}" ]]; then
        use_sudo="sudo"
        log_info "administrator privileges required for installation"
    fi
    
    if ! ${use_sudo} cp "${TEMP_BINARY_PATH}" "${INSTALL_DIR}/${CLIENT_NAME}"; then
        log_error "failed to copy binary to ${INSTALL_DIR}"
        exit 1
    fi
    
    if ! ${use_sudo} chmod +x "${INSTALL_DIR}/${CLIENT_NAME}"; then
        log_error "failed to make binary executable"
        exit 1
    fi
    
    log_success "installed ${CLIENT_NAME} to ${INSTALL_DIR}/${CLIENT_NAME}"
    
    rm -rf "$(dirname "${TEMP_BINARY_PATH}")"
}

check_path() {
    log_info "checking if ${INSTALL_DIR} is in PATH..."
    
    if echo ":${PATH}:" | grep -q ":${INSTALL_DIR}:"; then
        log_success "${INSTALL_DIR} is already in PATH"
        return 0
    else
        log_warning "${INSTALL_DIR} is not in PATH"
        return 1
    fi
}

provide_path_instructions() {
    log_info "to add ${INSTALL_DIR} to your PATH, add the following line to your shell configuration file:"
    echo
    echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
    echo
    
    local shell_name
    shell_name=$(basename "${SHELL}")
    
    case "${shell_name}" in
        bash)
            log_info "for bash, add to ~/.bashrc or ~/.bash_profile"
            ;;
        zsh)
            log_info "for zsh, add to ~/.zshrc"
            ;;
        fish)
            log_info "for fish, run: fish_add_path ${INSTALL_DIR}"
            ;;
        *)
            log_info "add to your shell's configuration file"
            ;;
    esac
    
    echo
    log_info "then restart your terminal or run: source ~/.bashrc (or appropriate config file)"
}

verify_installation() {
    log_info "verifying installation..."
    
    local binary_path="${INSTALL_DIR}/${CLIENT_NAME}"
    
    if [[ ! -f "${binary_path}" ]]; then
        log_error "binary not found at ${binary_path}"
        return 1
    fi
    
    if [[ ! -x "${binary_path}" ]]; then
        log_error "binary is not executable"
        return 1
    fi
    
    if "${binary_path}" version >/dev/null 2>&1; then
        log_success "installation verified successfully"
        
        log_info "installed version:"
        "${binary_path}" version
        
        return 0
    else
        log_warning "binary installed but version check failed"
        log_warning "this might be normal if the binary expects different arguments"
        return 1
    fi
}

main() {
    echo "funnel installation script"
    echo "========================="
    echo
    
    parse_arguments "$@"
    detect_platform
    check_dependencies
    get_latest_release
    download_binary
    install_binary
    
    echo
    echo "installation completed!"
    echo "======================"
    
    if ! check_path; then
        echo
        provide_path_instructions
    fi
    
    echo
    verify_installation
    
    echo
    log_success "installation complete! you can now use '${CLIENT_NAME}' command"
    
    if ! check_path; then
        log_info "note: you may need to restart your terminal or update your PATH"
    fi
}

main "$@" 
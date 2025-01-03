#!/bin/bash
set -e

# Setup logging
LOGDIR="logs"
LOGFILE="${LOGDIR}/verify_tests_$(date '+%Y%m%d_%H%M%S').log"
mkdir -p "$LOGDIR"

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOGFILE"
}

log "=== Test Verification Started ==="
log "System Info: $(uname -a)"

# Colors for output (with Windows compatibility)
if [ -t 1 ]; then
    # Check if running in Windows terminal that supports ANSI
    case "$OSTYPE" in
        msys*|cygwin*|mingw*)
            # Enable ANSI escape sequences in Windows
            export TERM=xterm-256color
            ;;
    esac
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS='Linux';;
        Darwin*)    OS='Mac';;
        CYGWIN*)    OS='Windows';;
        MINGW*)     OS='Windows';;
        MSYS*)      OS='Windows';;
        *)          OS='Unknown';;
    esac
    echo $OS
}

OS=$(detect_os)
log "Detected OS: $OS"

# Enhanced LLVM installation and verification
install_llvm() {
    if [ "$OS" = "Windows" ]; then
        log "Installing LLVM via Chocolatey..."
        if ! command -v choco &> /dev/null; then
            log "Installing Chocolatey package manager..."
            powershell -Command "Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))"
        fi
        
        # Remove existing LLVM if present
        choco uninstall -y llvm
        # Install specific LLVM version
        choco install -y llvm --version=12.0.1 --force
        
        # Set LLVM environment variables for Windows with proper path escaping
        LLVM_PATH="C:/Program Files/LLVM"
        ESCAPED_LLVM_PATH="${LLVM_PATH// /\\ }"
        
        if [ -d "$LLVM_PATH" ]; then
            export LLVM_SYS_120_PREFIX="$ESCAPED_LLVM_PATH"
            export PATH="$ESCAPED_LLVM_PATH/bin:$PATH"
            # Set additional environment variables
            export LIBCLANG_PATH="$ESCAPED_LLVM_PATH/bin"
            export CLANG_PATH="$ESCAPED_LLVM_PATH/bin/clang.exe"
            
            log "Set LLVM environment variables:"
            log "LLVM_SYS_120_PREFIX=$LLVM_SYS_120_PREFIX"
            log "LIBCLANG_PATH=$LIBCLANG_PATH"
            log "CLANG_PATH=$CLANG_PATH"
            
            # Verify LLVM binary accessibility
            if [ ! -f "$LLVM_PATH/bin/llvm-config.exe" ]; then
                log "ERROR: llvm-config.exe not found"
                exit 1
            fi
        else
            log "ERROR: LLVM installation directory not found"
            exit 1
        fi
        
        # Verify LLVM version
        LLVM_VERSION=$("$LLVM_PATH/bin/llvm-config.exe" --version || echo "unknown")
        log "Installed LLVM version: $LLVM_VERSION"
        if [[ ! $LLVM_VERSION == 12.* ]]; then
            log "ERROR: Wrong LLVM version. Expected 12.x.x, got $LLVM_VERSION"
            exit 1
        fi
    else
        log "Installing LLVM for Linux distribution..."
        if [ -f /etc/debian_version ]; then
            sudo apt-get update
            sudo apt-get install -y llvm-dev libclang-dev clang
        elif [ -f /etc/fedora-release ]; then
            sudo dnf install -y llvm-devel clang-devel
        elif [ -f /etc/arch-release ]; then
            sudo pacman -S --noconfirm llvm clang
        else
            log "ERROR: Unsupported distribution. Please install LLVM manually."
            echo -e "${RED}Unsupported distribution. Please install LLVM manually.${NC}"
            exit 1
        fi
    fi

    # Verify LLVM installation
    if command -v llvm-config &> /dev/null; then
        LLVM_VERSION=$(llvm-config --version | cut -d'.' -f1)
        if [ "$LLVM_VERSION" != "12" ]; then
            log "WARNING: LLVM version $LLVM_VERSION detected, but version 12 is required"
            echo -e "${YELLOW}WARNING: LLVM version $LLVM_VERSION detected, but version 12 is required${NC}"
            if [ "$OS" = "Windows" ]; then
                choco install -y llvm --version=12.0.1 --force
            fi
        fi
    else
        log "ERROR: LLVM installation failed"
        echo -e "${RED}ERROR: LLVM installation failed${NC}"
        exit 1
    fi
}

# Move LLVM check before any cargo commands
if ! command -v llvm-config &> /dev/null || [ -z "$LLVM_SYS_120_PREFIX" ]; then
    log "Installing/configuring LLVM..."
    echo -e "${YELLOW}Installing/configuring LLVM...${NC}"
    install_llvm
fi

# Convert paths based on OS
convert_path() {
    if [ "$OS" = "Windows" ]; then
        echo "${1//\//\\}"
    else
        echo "$1"
    fi
}

# Install cargo-tarpaulin with error handling
if ! command -v cargo-tarpaulin &> /dev/null; then
    log "Installing cargo-tarpaulin..."
    echo -e "${YELLOW}Installing cargo-tarpaulin...${NC}"
    cargo install cargo-tarpaulin || {
        log "ERROR: Failed to install cargo-tarpaulin. Please install manually."
        echo -e "${RED}Failed to install cargo-tarpaulin. Please install manually.${NC}"
        exit 1
    }
fi

# Check test files with OS-specific path handling
log "Checking required test files..."
required_files=(
    "tests/mod.rs"
    "tests/unit/lexer_tests.rs"
    "tests/unit/parser_tests.rs"
    "tests/integration/compiler_tests.rs"
    "tests/common/mod.rs"
)

for file in "${required_files[@]}"; do
    file_path=$(convert_path "$file")
    if [ ! -f "$file_path" ]; then
        log "ERROR: Missing test file: $file_path"
        echo -e "${RED}❌ Missing test file: $file_path${NC}"
        exit 1
    fi
done

log "All test files present"
echo -e "${GREEN}✓ All test files present${NC}"

# Run tests with proper error handling
run_command() {
    local cmd="$1"
    local desc="$2"
    
    log "Running: $desc"
    echo "Running $desc..."
    if ! eval "$cmd" >> "$LOGFILE" 2>&1; then
        log "ERROR: $desc failed"
        echo -e "${RED}❌ $desc failed${NC}"
        exit 1
    fi
    log "$desc completed successfully"
}

run_command "cargo fmt -- --check" "format check"
run_command "cargo clippy -- -D warnings" "clippy check"
run_command "RUST_BACKTRACE=1 cargo test --all-features" "test suite"

# Coverage analysis with Windows compatibility
log "Starting coverage analysis..."
echo "Running coverage analysis..."
if [ "$OS" = "Windows" ]; then
    coverage_dir=$(convert_path "coverage")
    mkdir -p "$coverage_dir"
fi

cargo tarpaulin --ignore-tests --out Xml --output-dir coverage || {
    log "ERROR: Coverage analysis failed"
    echo -e "${RED}❌ Coverage analysis failed${NC}"
    exit 1
}

# Parse coverage with Windows compatibility
if [ "$OS" = "Windows" ]; then
    coverage=$(powershell -Command "Select-Xml -Path coverage/cobertura.xml -XPath '//@line-rate' | ForEach-Object { `$_.Node.Value }")
else
    coverage=$(grep -Po 'line-rate="\K[^"]*' coverage/cobertura.xml)
fi

min_coverage=0.80
if (( $(echo "$coverage < $min_coverage" | bc -l) )); then
    log "ERROR: Coverage below threshold: ${coverage}% < ${min_coverage}%"
    echo -e "${RED}❌ Coverage below threshold: ${coverage}% < ${min_coverage}%${NC}"
    exit 1
fi

log "Test verification completed successfully"
log "Coverage: ${coverage}%"
echo -e "${GREEN}✅ All tests passed successfully${NC}"
echo -e "${GREEN}✅ Code coverage: ${coverage}%${NC}"

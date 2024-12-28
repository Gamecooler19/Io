#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}Checking system dependencies...${NC}"

# Check and install LLVM
if ! command -v llvm-config &> /dev/null; then
    echo -e "${YELLOW}Installing LLVM...${NC}"
    if [ -f /etc/debian_version ]; then
        sudo apt-get update
        sudo apt-get install -y llvm-dev libclang-dev clang
    elif [ -f /etc/fedora-release ]; then
        sudo dnf install -y llvm-devel clang-devel
    elif [ -f /etc/arch-release ]; then
        sudo pacman -S --noconfirm llvm clang
    else
        echo -e "${RED}Unsupported distribution. Please install LLVM manually.${NC}"
        exit 1
    fi
fi

# Install cargo-tarpaulin if not present
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-tarpaulin...${NC}"
    cargo install cargo-tarpaulin
fi

# Check if all test files exist
required_files=(
    "tests/mod.rs"
    "tests/unit/lexer_tests.rs"
    "tests/unit/parser_tests.rs"
    "tests/integration/compiler_tests.rs"
    "tests/common/mod.rs"
)

for file in "${required_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo -e "${RED}❌ Missing test file: $file${NC}"
        exit 1
    fi
done

echo -e "${GREEN}✓ All test files present${NC}"

# Run format check
echo "Running format check..."
cargo fmt -- --check || {
    echo -e "${RED}❌ Format check failed${NC}"
    exit 1
}

# Run clippy
echo "Running clippy..."
cargo clippy -- -D warnings || {
    echo -e "${RED}❌ Clippy check failed${NC}"
    exit 1
}

# Run tests with different configurations
echo "Running tests..."
RUST_BACKTRACE=1 cargo test --all-features || {
    echo -e "${RED}❌ Tests failed${NC}"
    exit 1
}

# Run coverage analysis
echo "Running coverage analysis..."
cargo tarpaulin --ignore-tests --out Xml --output-dir coverage || {
    echo -e "${RED}❌ Coverage analysis failed${NC}"
    exit 1
}

# Check minimum coverage threshold
coverage=$(grep -Po 'line-rate="\K[^"]*' coverage/cobertura.xml)
min_coverage=0.80
if (( $(echo "$coverage < $min_coverage" | bc -l) )); then
    echo -e "${RED}❌ Coverage below threshold: ${coverage}% < ${min_coverage}%${NC}"
    exit 1
fi

echo -e "${GREEN}✅ All tests passed successfully${NC}"
echo -e "${GREEN}✅ Code coverage: ${coverage}%${NC}"

#!/bin/bash
# Setup script for NeuroQuantumDB development environment
# This script sets up pre-commit hooks and development tools

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_status "Setting up NeuroQuantumDB development environment..."

# Check if we're in the correct repository
if [ ! -f "Cargo.toml" ] || ! grep -q "NeuroQuantumDB" "Cargo.toml" 2>/dev/null; then
    print_error "This script must be run from the NeuroQuantumDB repository root"
    exit 1
fi

# Check if Git is available
if ! command -v git &> /dev/null; then
    print_error "Git is not installed. Please install Git first."
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo is not installed. Please install Rust first:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Install required linting tools
print_status "Installing required development tools..."

tools=("cargo-audit" "cargo-deny" "cargo-machete" "cargo-tarpaulin")
for tool in "${tools[@]}"; do
    if ! command -v "$tool" &> /dev/null; then
        print_status "Installing $tool..."
        cargo install "$tool"
    else
        print_status "$tool is already installed"
    fi
done

# Copy pre-commit hook from hooks/ directory to .git/hooks/
print_status "Setting up pre-commit hooks..."

if [ ! -d ".git" ]; then
    print_error "Not a Git repository. Run 'git init' first."
    exit 1
fi

# Ensure the hooks directory exists
mkdir -p .git/hooks

# Copy the pre-commit hook
if [ -f "hooks/pre-commit" ]; then
    cp "hooks/pre-commit" ".git/hooks/pre-commit"
    chmod +x ".git/hooks/pre-commit"
    print_success "Pre-commit hook installed"
else
    print_error "Pre-commit hook file not found at hooks/pre-commit"
    exit 1
fi

# Copy other hooks if they exist
for hook in "hooks/"*; do
    if [ -f "$hook" ] && [ "$(basename "$hook")" != "pre-commit" ]; then
        hook_name=$(basename "$hook")
        cp "$hook" ".git/hooks/$hook_name"
        chmod +x ".git/hooks/$hook_name"
        print_status "Installed hook: $hook_name"
    fi
done

# Set up Git configuration for better development experience
print_status "Configuring Git settings..."

git config --local core.autocrlf false
git config --local core.eol lf
git config --local pull.rebase true
git config --local fetch.prune true
git config --local diff.colorMoved zebra

print_success "Git configuration updated"

# Test the setup
print_status "Testing the setup..."

# Run a quick format check
if cargo fmt --all -- --check &> /dev/null; then
    print_success "Code formatting check passed"
else
    print_warning "Code formatting needs attention. Run 'cargo fmt --all' to fix."
fi

# Run a quick clippy check
if cargo clippy --workspace --all-targets --all-features -- -D warnings &> /dev/null; then
    print_success "Clippy check passed"
else
    print_warning "Clippy found issues. Run 'cargo clippy --workspace --all-targets --all-features -- -D warnings' for details."
fi

print_success "Development environment setup complete! 🎉"
print_status "Next steps:"
echo "  1. Run 'make lint' to verify all linting tools work"
echo "  2. Run 'make test' to ensure all tests pass"
echo "  3. Make your first commit to test the pre-commit hooks"
echo ""
print_status "The pre-commit hooks will now run automatically before every commit."

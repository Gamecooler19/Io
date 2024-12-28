# Installing Io

This guide explains the most efficient ways to install Io. Choose the method that best suits your environment and workflow.

## System Requirements

- 64-bit operating system
- Git (for source installation)
- Rust toolchain (for building from source)

## Installation Methods

### From Binary Releases

```bash
curl -sL https://io-lang.org/install.sh | sh
```

### From Source

```bash
git clone https://github.com/io-lang/io.git
cd io
cargo install --path .
```

## Verifying Installation

```bash
ioc --version
```
If the above command shows the installed version, your Io setup is ready.

## Version Management
If you need multiple Io versions, consider using a version manager tool for parallel installations.

## Troubleshooting
- Ensure your PATH includes the Io binaries.
- Check for network issues when installing from source.

## Next Steps

See the [Getting Started Guide](./first-steps.md) for your first Io program.

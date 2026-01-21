# GitHub Actions Workflow

This repository uses GitHub Actions to automatically build and publish multi-architecture Docker images.

## Workflow Overview

The workflow consists of two main jobs:

### 1. Build Binaries (`build-binaries`)

This job builds the Rust binary for two target architectures:

- **x86_64-unknown-linux-musl** (amd64): Standard cargo build
- **aarch64-unknown-linux-musl** (arm64): Uses `cargo-zigbuild` for cross-compilation

The binaries are statically linked using musl libc, which produces a standalone executable with no runtime dependencies.

**Why cargo-zigbuild?**
Cross-compiling Rust to ARM64 can be tricky. `cargo-zigbuild` uses the Zig compiler as a linker, which makes cross-compilation much simpler and more reliable without needing a full cross-compilation toolchain.

### 2. Build and Push Docker Image (`build-and-push-image`)

This job:
1. Downloads the pre-built binaries from the previous job
2. Uses a multi-stage Dockerfile to create minimal Alpine-based images
3. Builds images for both `linux/amd64` and `linux/arm64` platforms
4. Pushes to GitHub Container Registry (ghcr.io)

## Triggering the Workflow

The workflow triggers on:

- **Push to `main` branch**: Builds and pushes with `main` tag
- **Pull requests**: Builds only (doesn't push)
- **Git tags starting with `v`**: Creates versioned releases (e.g., `v1.0.0`, `v1.2.3`)

## Image Tags

The workflow automatically creates the following tags:

- `main` - Latest build from main branch
- `v1.2.3` - Full semantic version
- `v1.2` - Major.minor version
- `v1` - Major version only
- `sha-abc1234` - Git commit SHA

## Setup Requirements

To use this workflow in your repository:

1. **Enable GitHub Actions**
   - Go to Settings → Actions → General
   - Ensure "Allow all actions and reusable workflows" is selected

2. **Enable GitHub Packages**
   - Go to Settings → Actions → General → Workflow permissions
   - Select "Read and write permissions"
   - Check "Allow GitHub Actions to create and approve pull requests"

3. **No secrets required!**
   - The workflow uses `GITHUB_TOKEN` which is automatically provided
   - Images are published to GitHub Container Registry (ghcr.io)

## Local Testing

To test the workflow locally before pushing:

### Build binaries locally:

```bash
# x86_64
cargo build --release --target x86_64-unknown-linux-musl

# aarch64 (requires cargo-zigbuild)
pip install ziglang
cargo install cargo-zigbuild
cargo zigbuild --release --target aarch64-unknown-linux-musl
```

### Test Docker build:

```bash
# Prepare binaries for Docker
mkdir -p binaries/amd64 binaries/arm64
cp target/x86_64-unknown-linux-musl/release/p1-prometheus-exporter-rs binaries/amd64/
cp target/aarch64-unknown-linux-musl/release/p1-prometheus-exporter-rs binaries/arm64/

# Build Docker image
docker buildx build --platform linux/amd64,linux/arm64 -t p1-exporter:test .
```

## Caching

The workflow uses GitHub Actions cache for:

- Cargo registry and index
- Cargo build artifacts
- Docker layer cache

This significantly speeds up subsequent builds.

## Workflow Files

- `.github/workflows/build-and-publish.yml` - Main CI/CD workflow
- `Dockerfile` - Multi-arch Dockerfile using pre-built binaries
- `.dockerignore` - Excludes unnecessary files from Docker context

## Troubleshooting

### Build fails on cargo-zigbuild

Make sure you have Python and pip available. The workflow installs ziglang via pip before installing cargo-zigbuild.

### Permission denied when pushing to ghcr.io

Ensure that:
1. GitHub Actions has write permissions in repository settings
2. The repository owner has enabled GitHub Packages
3. You're not trying to push on a PR (PRs only build, don't push)

### Binary not found in Docker build

The workflow downloads artifacts from the build-binaries job. Ensure:
1. Both build-binaries jobs completed successfully
2. The artifact upload step succeeded
3. The binary name matches: `p1-prometheus-exporter-rs`

## Publishing a Release

To publish a new versioned release:

```bash
# Tag the commit
git tag v1.0.0
git push origin v1.0.0

# The workflow will automatically:
# - Build binaries for both architectures
# - Create Docker images with tags: v1.0.0, v1.0, v1, latest
# - Push to ghcr.io
```

## Image Usage

After the workflow completes, pull and use the image:

```bash
# Pull the image
docker pull ghcr.io/thijslemmens/p1-prometheus-exporter:latest

# Run it
docker run -d \
  --name p1-exporter \
  --device=/dev/ttyUSB0 \
  -p 9292:9292 \
  ghcr.io/thijslemmens/p1-prometheus-exporter:latest
```

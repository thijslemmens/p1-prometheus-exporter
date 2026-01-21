# Deployment Setup Checklist

This document provides a step-by-step guide to set up automated builds and deployments for the P1 Prometheus Exporter.

## Prerequisites

- [ ] GitHub account with a repository for this project
- [ ] Git installed locally
- [ ] Docker installed (for local testing)

## Initial Setup

### 1. Repository Setup

- [ ] Push your code to GitHub
  ```bash
  git remote add origin https://github.com/YOUR-USERNAME/YOUR-REPO-NAME.git
  git push -u origin main
  ```

- [ ] Replace `OWNER/REPO` placeholders in the following files:
  - [ ] `README.md` - Update badge and docker pull examples
  - [ ] `docker-compose.yml` - Update image name
  - [ ] `.github/workflows/README.md` - Update examples

### 2. Enable GitHub Actions

- [ ] Go to your repository on GitHub
- [ ] Navigate to **Settings** → **Actions** → **General**
- [ ] Under "Actions permissions", select **"Allow all actions and reusable workflows"**
- [ ] Click **Save**

### 3. Enable GitHub Packages/Container Registry

- [ ] Still in **Settings** → **Actions** → **General**
- [ ] Scroll to **"Workflow permissions"**
- [ ] Select **"Read and write permissions"**
- [ ] Check **"Allow GitHub Actions to create and approve pull requests"**
- [ ] Click **Save**

### 4. Enable GitHub Container Registry (Public Access - Optional)

By default, GitHub Container Registry images are private. To make them public:

- [ ] Go to your GitHub profile
- [ ] Click on **Packages** tab
- [ ] Find your `p1-prometheus-exporter-rs` package (after first build)
- [ ] Click **Package settings**
- [ ] Scroll to **"Danger Zone"**
- [ ] Click **"Change visibility"**
- [ ] Select **"Public"** and confirm

## First Build

### Trigger the Workflow

- [ ] Make a commit and push to main:
  ```bash
  git add .
  git commit -m "Setup CI/CD workflow"
  git push origin main
  ```

- [ ] Go to **Actions** tab in your repository
- [ ] Watch the **"Build and Publish"** workflow run
- [ ] Verify both jobs complete successfully:
  - [ ] `build-binaries` (runs twice: x86_64 and aarch64)
  - [ ] `build-and-push-image`

### Verify the Build

- [ ] Check that the Docker image was published:
  - Go to your repository main page
  - Look for **"Packages"** in the right sidebar
  - Click on the package to see available tags

- [ ] Pull and test the image locally:
  ```bash
  docker pull ghcr.io/YOUR-USERNAME/YOUR-REPO:main
  docker run --rm ghcr.io/YOUR-USERNAME/YOUR-REPO:main --help
  ```

## Create a Release

### Tag a Version

- [ ] Create and push a version tag:
  ```bash
  git tag v1.0.0
  git push origin v1.0.0
  ```

- [ ] Verify the workflow runs for the tag
- [ ] Check that multiple version tags are created:
  - [ ] `v1.0.0` (full version)
  - [ ] `v1.0` (major.minor)
  - [ ] `v1` (major only)

### Create GitHub Release (Optional)

- [ ] Go to **Releases** in your repository
- [ ] Click **"Create a new release"**
- [ ] Select your tag (`v1.0.0`)
- [ ] Add release notes
- [ ] Publish the release

## Deploy to Production

### Using Docker

- [ ] On your target machine, pull the image:
  ```bash
  docker pull ghcr.io/YOUR-USERNAME/YOUR-REPO:v1.0.0
  ```

- [ ] Run the container:
  ```bash
  docker run -d \
    --name p1-exporter \
    --device=/dev/ttyUSB0 \
    --restart unless-stopped \
    -p 9292:9292 \
    ghcr.io/YOUR-USERNAME/YOUR-REPO:v1.0.0
  ```

### Using Docker Compose

- [ ] Copy `docker-compose.yml` to your server
- [ ] Update the image name in the file
- [ ] Start the service:
  ```bash
  docker-compose up -d
  ```

- [ ] Check logs:
  ```bash
  docker-compose logs -f p1-exporter
  ```

- [ ] Verify metrics are available:
  ```bash
  curl http://localhost:9292/metrics
  ```

## Configure Monitoring

### Prometheus

- [ ] Copy `prometheus.yml.example` to `prometheus.yml`
- [ ] Update the configuration with your setup
- [ ] Add the P1 exporter as a scrape target
- [ ] Reload Prometheus configuration

### Grafana (Optional)

- [ ] Create a Grafana dashboard for P1 metrics
- [ ] Import or create panels for:
  - [ ] Current power consumption/delivery
  - [ ] Total energy consumed/delivered
  - [ ] Gas consumption
  - [ ] Voltage and current readings

## Maintenance

### Update the Application

- [ ] Make your code changes
- [ ] Commit and push to main (creates `main` tag)
- [ ] Or create a new version tag for releases:
  ```bash
  git tag v1.1.0
  git push origin v1.1.0
  ```

- [ ] On your server, pull the new image:
  ```bash
  docker-compose pull
  docker-compose up -d
  ```

### Monitor Build Status

- [ ] Add the build badge to your README (already included)
- [ ] Check the **Actions** tab regularly for build failures
- [ ] Enable GitHub notifications for failed workflows:
  - Go to **Settings** → **Notifications**
  - Enable email notifications for Actions

## Troubleshooting

### Build Fails

- [ ] Check the Actions tab for error logs
- [ ] Common issues:
  - [ ] Cargo.toml syntax errors
  - [ ] Missing dependencies
  - [ ] Test failures
  - [ ] cargo-zigbuild installation issues

### Permission Errors

- [ ] Verify workflow permissions are set correctly
- [ ] Check that `GITHUB_TOKEN` has write access
- [ ] Ensure you're not building on a fork (forks have restricted permissions)

### Docker Image Not Available

- [ ] Check that the workflow completed successfully
- [ ] Verify the image was pushed (not just built)
- [ ] Ensure the package visibility is set correctly (public/private)
- [ ] Check you're using the correct image name:
  ```
  ghcr.io/YOUR-USERNAME/YOUR-REPO:TAG
  ```

### Serial Port Issues

**See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for detailed serial port permission solutions.**

Quick fixes:
- [ ] Ensure the device exists: `ls -l /dev/ttyUSB0`
- [ ] Check device permissions: `sudo chmod 666 /dev/ttyUSB0`
- [ ] Or add user to dialout group: `sudo usermod -a -G dialout $USER`
- [ ] Verify docker-compose.yml includes `group_add: dialout`
- [ ] Verify the correct DSMR mode (new vs old)

## Support

- [ ] Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues and solutions
- [ ] Check existing issues in the repository
- [ ] Read `.github/workflows/README.md` for CI/CD details
- [ ] Review GitHub Actions documentation
- [ ] Run the diagnostic script from TROUBLESHOOTING.md

## Checklist Complete! 🎉

Once you've completed all the steps above, your P1 Prometheus Exporter should be:
- ✅ Automatically building on every push
- ✅ Publishing multi-architecture Docker images
- ✅ Running in production
- ✅ Exporting metrics to Prometheus

Happy monitoring! 📊
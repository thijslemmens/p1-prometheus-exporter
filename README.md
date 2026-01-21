# P1 Prometheus exporter

[![Build and Publish](https://github.com/thijslemmens/p1-prometheus-exporter/actions/workflows/build-and-publish.yml/badge.svg)](https://github.com/thijslemmens/p1-prometheus-exporter/actions/workflows/build-and-publish.yml)

This is a Prometheus exporter
for smart meters. It also retrieves the gas meter readings.

Requires a P1 cable connected to your computer.

## Usage

### Using Docker (recommended)

Docker images are automatically built for both `amd64` and `arm64` architectures and published to GitHub Container Registry.

Pull the latest image (replace `OWNER/REPO` with your GitHub username and repository name):
```bash
docker pull ghcr.io/OWNER/REPO:latest

# Example:
# docker pull ghcr.io/john/p1-prometheus-exporter:latest
```

Run the exporter with your P1 serial device:
```bash
docker run -d \
  --name p1-exporter \
  --device=/dev/ttyUSB0 \
  -p 9292:9292 \
  ghcr.io/OWNER/REPO:latest \
  --serial-port /dev/ttyUSB0
```

For DSMR 2.x/3.x meters (older meters with 7E1 serial settings):
```bash
docker run -d \
  --name p1-exporter \
  --device=/dev/ttyUSB0 \
  -p 9292:9292 \
  ghcr.io/OWNER/REPO:latest \
  --serial-port /dev/ttyUSB0 \
  --dsmr old
```

Enable verbose logging (shows every received serial line):
```bash
docker run -d \
  --name p1-exporter \
  --device=/dev/ttyUSB0 \
  -p 9292:9292 \
  ghcr.io/OWNER/REPO:latest \
  --serial-port /dev/ttyUSB0 \
  --verbose
```

The metrics will be available at `http://localhost:9292/metrics`.

### Command-line options

- `--port <PORT>` - Port for the Prometheus exporter (default: 9292)
- `--serial-port <DEVICE>` - Serial port device (default: /dev/ttyUSB0)
- `--dsmr <MODE>` - DSMR mode: `new` for DSMR 4.0+ (8N1) or `old` for DSMR 2.x/3.x (7E1) (default: new)
- `--verbose` / `-v` - Enable debug logging (shows all received serial messages)

You can also control logging via the `RUST_LOG` environment variable:
```bash
docker run -d \
  --name p1-exporter \
  --device=/dev/ttyUSB0 \
  -p 9292:9292 \
  -e RUST_LOG=debug \
  ghcr.io/OWNER/REPO:latest
```

### Using Docker Compose

For easier deployment, use the included `docker-compose.yml`:

1. Copy the example file and update the image name:
   ```bash
   cp docker-compose.yml docker-compose.local.yml
   # Edit docker-compose.local.yml to replace OWNER/REPO with your GitHub username/repo
   ```

2. Start the service:
   ```bash
   docker-compose -f docker-compose.local.yml up -d
   ```

3. View logs:
   ```bash
   docker-compose -f docker-compose.local.yml logs -f
   ```

4. Stop the service:
   ```bash
   docker-compose -f docker-compose.local.yml down
   ```

## Building

1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Run: `cargo run` or build for release `cargo build --release`

### Debian package

To build a Debian package:

```
$ cargo install cargo-deb
$ cargo deb
```

### Building Docker Images

The project uses GitHub Actions to automatically build multi-architecture Docker images. The workflow:

1. Builds Rust binaries for both `x86_64` and `aarch64` architectures
2. Uses `cargo zigbuild` for cross-compilation to ARM64
3. Creates a minimal Alpine-based Docker image with the pre-built binaries
4. Publishes to GitHub Container Registry

To set up automated builds:

1. Enable GitHub Actions in your repository
2. Ensure GitHub Packages permissions are enabled (Settings → Actions → General → Workflow permissions → Read and write permissions)
3. Push to `main` branch or create a tag starting with `v` (e.g., `v1.0.0`)

The workflow will automatically:
- Build on every push to `main`
- Build on every PR
- Create versioned tags when you push a git tag

Manual Docker build (local):
```bash
# Build for current architecture
docker build -t p1-exporter .

# Build for specific architecture
docker buildx build --platform linux/amd64 -t p1-exporter:amd64 .
docker buildx build --platform linux/arm64 -t p1-exporter:arm64 .
```

Note: For local multi-arch builds, you'll need to build the Rust binaries first:
```bash
# x86_64
cargo build --release --target x86_64-unknown-linux-musl
mkdir -p binaries/amd64
cp target/x86_64-unknown-linux-musl/release/p1-prometheus-exporter-rs binaries/amd64/

# aarch64 (requires cargo-zigbuild)
cargo install cargo-zigbuild
cargo zigbuild --release --target aarch64-unknown-linux-musl
mkdir -p binaries/arm64
cp target/aarch64-unknown-linux-musl/release/p1-prometheus-exporter-rs binaries/arm64/
```

## Implementation Details

The P1 port carries is a pretty simple protocol, called DSMR (Dutch Smart Meter Requirements). On each line it provides
an address for a metric and the value of that metric. Every ten seconds you get a new reading over the serial line. An
explanation of what each address means can be found in the https://github.com/energietransitie/dsmr-info project.

The Kamstrup 162JXC adheres to DSMR 3.0. This project will probably also work with other DSMR 3.0 meters.

Below is a typical message:

```
/KMP5 KA6UXXXXXXXXXXXX
0-0:96.1.1(XXXXXX)
1-0:1.8.1(20203.975*kWh)
1-0:1.8.2(18247.900*kWh)
1-0:2.8.1(00238.368*kWh)
1-0:2.8.2(00532.631*kWh)
0-0:96.14.0(0001)
1-0:1.7.0(0000.29*kW)
1-0:2.7.0(0000.00*kW)
0-0:96.13.1()
0-0:96.13.0()
0-1:24.1.0(3)
0-1:96.1.0(XXXXXX)
0-1:24.3.0(240914210000)(08)(60)(1)(0-1:24.2.1)(m3)
(14094.865)
!
```

This results in the following Prometheus export:

```
# HELP kamstrup_162jxc_p1_actual_power_delivered_watts Actual electricity power delivered (+P) in 1 Watt resolution (1-0:1.7.0)
# TYPE kamstrup_162jxc_p1_actual_power_delivered_watts gauge
kamstrup_162jxc_p1_actual_power_delivered_watts 290
# HELP kamstrup_162jxc_p1_actual_power_received_watts Actual electricity power received (-P) in 1 Watt resolution (1-0:2.7.0)
# TYPE kamstrup_162jxc_p1_actual_power_received_watts gauge
kamstrup_162jxc_p1_actual_power_received_watts 0
# HELP kamstrup_162jxc_p1_total_energy_delivered_watthours Total electricity energy delivered (+P) in 1 Watthour resolution (1-0:1.8.1 / 1-0:1.8.2)
# TYPE kamstrup_162jxc_p1_total_energy_delivered_watthours counter
kamstrup_162jxc_p1_total_energy_delivered_watthours{meter="1"} 20203975
kamstrup_162jxc_p1_total_energy_delivered_watthours{meter="2"} 18247900
# HELP kamstrup_162jxc_p1_total_energy_received_watthours Total electricity energy received (-P) in 1 Watthour resolution (1-0:2.8.1 / 1-0:2.8.2)
# TYPE kamstrup_162jxc_p1_total_energy_received_watthours counter
kamstrup_162jxc_p1_total_energy_received_watthours{meter="1"} 238368
kamstrup_162jxc_p1_total_energy_received_watthours{meter="2"} 532631
# HELP kamstrup_162jxc_p1_total_gas_delivered_m3 Total gas delivered in m3 (0-1:24.3.0)
# TYPE kamstrup_162jxc_p1_total_gas_delivered_m3 counter
kamstrup_162jxc_p1_total_gas_delivered_m3 14094.865
```

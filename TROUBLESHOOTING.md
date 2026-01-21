# Troubleshooting Guide

This guide helps resolve common issues when running the P1 Prometheus Exporter.

## Serial Port Permission Issues

### Problem: "Permission denied" when accessing /dev/ttyUSB0

```
ERROR p1_prometheus_exporter_rs] Failed to open serial port /dev/ttyUSB0: Permission denied
```

This is the most common issue when running in Docker. The container doesn't have permission to access the serial device.

### Solutions (in order of preference):

#### Solution 1: Add dialout group to Docker container (Recommended)

The `docker-compose.yml` already includes `group_add: dialout`, but you need to ensure the dialout group exists and has the right permissions.

1. Check your dialout group ID:
   ```bash
   getent group dialout
   ```
   Output example: `dialout:x:20:thijs`

2. If the GID is not 20, update `docker-compose.yml`:
   ```yaml
   group_add:
     - "997"  # Replace with your actual dialout GID
   ```

3. Ensure your user is in the dialout group:
   ```bash
   sudo usermod -a -G dialout $USER
   ```

4. Restart Docker to apply group changes:
   ```bash
   docker-compose down
   docker-compose up -d
   ```

#### Solution 2: Set serial device permissions

Make the serial device accessible to all users (less secure, but works):

```bash
sudo chmod 666 /dev/ttyUSB0
```

**Note:** This permission will reset after unplugging/replugging the device or rebooting.

To make it permanent, create a udev rule:

```bash
# Create udev rule
sudo nano /etc/udev/rules.d/99-usb-serial.rules

# Add this line:
KERNEL=="ttyUSB[0-9]*", MODE="0666"

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

#### Solution 3: Use privileged mode (Not recommended for production)

Add to your `docker-compose.yml`:

```yaml
services:
  p1-exporter:
    privileged: true
    # ... rest of config
```

**Warning:** This gives the container elevated privileges and should only be used for testing.

#### Solution 4: Run outside Docker

If all else fails, run the binary directly on your host:

```bash
# Install on host
cargo build --release
sudo cp target/release/p1-prometheus-exporter-rs /usr/local/bin/

# Add your user to dialout group
sudo usermod -a -G dialout $USER

# Log out and back in, then run
p1-prometheus-exporter-rs --serial-port /dev/ttyUSB0
```

## Device Not Found Issues

### Problem: /dev/ttyUSB0 doesn't exist

```
ERROR p1_prometheus_exporter_rs] Failed to open serial port /dev/ttyUSB0: No such file or directory
```

#### Check if the device exists:

```bash
ls -l /dev/ttyUSB*
ls -l /dev/ttyACM*
ls -l /dev/serial/by-id/
```

#### Find your serial device:

```bash
# Before plugging in the P1 cable:
ls /dev/tty* > /tmp/before.txt

# After plugging in:
ls /dev/tty* > /tmp/after.txt

# See the difference:
diff /tmp/before.txt /tmp/after.txt
```

#### Common device names:

- `/dev/ttyUSB0` - Most USB-to-serial adapters
- `/dev/ttyACM0` - Some USB devices
- `/dev/serial/by-id/usb-...` - Persistent device name (recommended)

#### Update docker-compose.yml with your device:

```yaml
devices:
  - /dev/ttyACM0:/dev/ttyACM0  # Update here

command:
  - --serial-port
  - /dev/ttyACM0  # And here
```

## DSMR Version Issues

### Problem: Lines are truncated or garbled

```
Received message: 01000000W)(250119174500W)(05.436*kW)...
```

This indicates wrong serial port settings. Modern meters use DSMR 4.0+ (8N1), older meters use DSMR 2.x/3.x (7E1).

#### Solution: Specify DSMR version

For **DSMR 4.0+ meters** (default - most common):
```yaml
command:
  - --serial-port
  - /dev/ttyUSB0
  - --dsmr
  - new
```

For **DSMR 2.x/3.x meters** (older):
```yaml
command:
  - --serial-port
  - /dev/ttyUSB0
  - --dsmr
  - old
```

## No Metrics / Missing Data

### Problem: Metrics endpoint returns no data or missing metrics

#### Check if data is being received:

Enable verbose logging:

```bash
docker logs -f p1-prometheus-exporter
```

Or run with verbose flag:

```yaml
command:
  - --serial-port
  - /dev/ttyUSB0
  - --verbose  # Add this
```

You should see lines like:
```
[DEBUG] Received message: 1-0:1.7.0(00.957*kW)
[DEBUG] Received message: 1-0:2.7.0(00.000*kW)
```

#### Check the metrics endpoint:

```bash
curl http://localhost:9292/metrics
```

Should return Prometheus metrics like:
```
# HELP p1_actual_power_delivered_watts ...
# TYPE p1_actual_power_delivered_watts gauge
p1_actual_power_delivered_watts 957
```

#### Common causes:

1. **Wrong DSMR version** - Try switching between `--dsmr new` and `--dsmr old`
2. **Cable not connected properly**
3. **Meter P1 port not enabled** - Some meters need activation
4. **Wrong baud rate** - The exporter uses 115200, which is standard for DSMR

## Docker Issues

### Problem: Container keeps restarting

Check logs:
```bash
docker logs p1-prometheus-exporter
```

Check container status:
```bash
docker ps -a
```

### Problem: Can't pull image from ghcr.io

Ensure the image name is correct:
```bash
docker pull ghcr.io/USERNAME/REPO:latest
```

If the image is private, authenticate:
```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin
```

### Problem: Architecture mismatch

The images are built for `amd64` (x86_64) and `arm64` (aarch64). 

Check your architecture:
```bash
uname -m
# x86_64 = amd64
# aarch64 = arm64
# armv7l = Need to build for armv7 (Raspberry Pi 3 and older)
```

For Raspberry Pi 3 (armv7), you'll need to build locally or add armv7 to the CI/CD workflow.

## Network/Port Issues

### Problem: Can't access metrics on port 9292

Check if the port is bound:
```bash
netstat -tuln | grep 9292
# or
ss -tuln | grep 9292
```

Check if the container is running:
```bash
docker ps | grep p1-exporter
```

Check Docker port mapping:
```bash
docker port p1-prometheus-exporter
```

Try accessing from inside the container:
```bash
docker exec p1-prometheus-exporter wget -O- http://localhost:9292/metrics
```

## Raspberry Pi Specific Issues

### USB Power Issues

Some Raspberry Pi USB ports don't provide enough power for USB serial adapters.

Try:
1. Use a powered USB hub
2. Use a different USB port
3. Check `dmesg` for USB errors:
   ```bash
   dmesg | grep -i usb
   ```

### Memory Issues

If the container is killed randomly, check memory:
```bash
free -h
docker stats p1-prometheus-exporter
```

## Getting Help

If you're still stuck:

1. Check existing issues: https://github.com/OWNER/REPO/issues
2. Gather diagnostic information:
   ```bash
   # System info
   uname -a
   docker --version
   
   # Serial device info
   ls -l /dev/ttyUSB*
   dmesg | tail -20
   
   # Container logs
   docker logs p1-prometheus-exporter --tail 50
   
   # Test metrics endpoint
   curl http://localhost:9292/metrics
   ```

3. Create a new issue with this information

## Quick Diagnostic Script

Save this as `diagnose.sh` and run it:

```bash
#!/bin/bash
echo "=== System Info ==="
uname -a
echo ""

echo "=== Docker Version ==="
docker --version
echo ""

echo "=== Serial Devices ==="
ls -l /dev/ttyUSB* /dev/ttyACM* 2>/dev/null || echo "No serial devices found"
echo ""

echo "=== Dialout Group ==="
getent group dialout
groups $USER
echo ""

echo "=== Container Status ==="
docker ps -a | grep p1-exporter
echo ""

echo "=== Recent Logs ==="
docker logs p1-prometheus-exporter --tail 20 2>/dev/null || echo "Container not running"
echo ""

echo "=== Port Check ==="
netstat -tuln | grep 9292 || echo "Port 9292 not listening"
echo ""

echo "=== Metrics Test ==="
curl -s http://localhost:9292/metrics | head -20 || echo "Cannot reach metrics endpoint"
```

Run it:
```bash
chmod +x diagnose.sh
./diagnose.sh
```

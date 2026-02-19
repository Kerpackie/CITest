# Watlow PM8 Modbus RTU Simulator

A high-fidelity Modbus RTU simulator for Watlow PM8 temperature controllers, written in Rust. This simulator emulates the behavior of a real Watlow PM8 device, including thermal physics simulation for realistic Process Value (PV) responses to Setpoint (SP) changes.

## Features

- **Realistic Physics Simulation**: Simulates thermal inertia with heating/cooling curves, ambient temperature floor, and sensor noise
- **Watlow PM8 Register Compatibility**: Supports official Watlow register addresses (360, 2322, 7101)
- **Multiple Data Formats**: Provides both integer (scaled x10) and IEEE 754 32-bit float representations
- **Modbus RTU Server**: Full implementation using serial port communication
- **Modbus RTU Client**: Included test client for polling and setpoint control
- **Cross-Platform**: Works on Windows (COM ports), Linux (/dev/ttyUSB*, /dev/pts/*), and macOS

## Components

### 1. Simulator Server (`watlow_simulator`)
The main simulator that acts as a Modbus RTU slave device, responding to read and write requests.

### 2. Client Logger (`watlow_simulator_client`)
A polling client that continuously reads process values and setpoint, with optional setpoint writing capability.

## Installation

### Prerequisites
- Rust 1.70+ (2024 edition)
- Serial port hardware or virtual serial port software

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd watlow_simulator

# Build both binaries
cargo build --release

# Binaries will be in target/release/
# - watlow_simulator.exe (or watlow_simulator on Unix)
# - watlow_simulator_client.exe (or watlow_simulator_client on Unix)
```

## Usage

### Running the Simulator Server

```bash
# Windows example
watlow_simulator --port COM4 --baud 9600

# Linux example
watlow_simulator --port /dev/ttyUSB0 --baud 9600

# Virtual serial port example
watlow_simulator --port /dev/pts/3 --baud 9600
```

**Options:**
- `--port`, `-p`: Serial port name (required)
- `--baud`, `-b`: Baud rate (default: 9600)

### Running the Client Logger

```bash
# Basic polling
watlow_simulator_client --port COM5 --baud 9600 --unit-id 1

# Set a specific setpoint at startup, then poll
watlow_simulator_client --port COM5 --baud 9600 --unit-id 1 --set-sp 75.5

# Poll every 2 seconds instead of default 1 second
watlow_simulator_client --port COM5 --baud 9600 --interval 2000
```

**Options:**
- `--port`, `-p`: Serial port name (required)
- `--baud`, `-b`: Baud rate (default: 9600)
- `--unit-id`, `-u`: Modbus slave address (default: 1)
- `--set-sp`: Optional setpoint to write at startup
- `--interval`, `-i`: Polling interval in milliseconds (default: 1000)

## Supported Modbus Registers

The simulator implements the following Watlow PM8 register mappings:

| Register | Format | Description | Access |
|----------|--------|-------------|--------|
| **7101** | INT16 (×10) | Process Value (PV) - Temperature | Read |
| **360-361** | FLOAT32 (IEEE 754) | Process Value (PV) - Temperature | Read |
| **2322** | INT16 (×10) | Setpoint 1 | Read/Write |
| **300** | INT16 (×10) | Active Setpoint (legacy) | Read/Write |

### Data Format Notes

- **Integer Registers**: Values are scaled by 10. For example:
  - `250` = 25.0°C
  - `1055` = 105.5°C
  
- **Float Registers**: Stored as IEEE 754 32-bit float across two consecutive 16-bit registers:
  - High word (register 360)
  - Low word (register 361)

## Supported Modbus Function Codes

- **FC 03**: Read Holding Registers
- **FC 06**: Write Single Register
- **FC 16**: Write Multiple Registers

## Physics Simulation

The simulator includes a realistic thermal model that runs in a background thread:

- **Heating Rate**: +0.08°C per 200ms cycle when SP > PV
- **Cooling Rate**: -0.02°C per 200ms cycle when PV > ambient
- **Ambient Temperature**: 22.0°C (acts as temperature floor)
- **Sensor Noise**: ±0.01°C jitter for realism
- **Initial Conditions**: PV = 22.1°C, SP = 50.0°C

### Example Behavior

1. Set SP to 100°C → PV gradually heats from ambient (~22°C) to 100°C
2. Set SP to 25°C → PV cools from current temperature toward 25°C
3. Set SP to 20°C → PV cools only to ambient temperature (~22°C)

## Example Session

### Terminal 1: Start the Simulator
```bash
$ watlow_simulator --port COM4 --baud 9600

--- Watlow PM8 RTU Simulator ---
Listening on: COM4 @ 9600 baud
Available Registers:
  - Process Value (PV): 7101 (Int x10), 360 (32-bit Float)
  - Setpoint (SP):     2322 (Int x10), 300 (Int x10)
```

### Terminal 2: Connect with Client
```bash
$ watlow_simulator_client --port COM5 --baud 9600 --unit-id 1 --set-sp 80.0

--- Watlow PM8 Modbus Client ---
Connecting to: COM5 @ 9600 baud (Slave ID: 1)
>>> WRITING SETPOINT: 80.0°C
>>> Setpoint Write Success

Starting Logger (Ctrl+C to stop)...
Timestamp                 | PV (Int)   | PV (F32)   | SP (Read)
-----------------------------------------------------------------
2026-02-19 14:32:01       | 22.3       | 22.3145    | 80.0
2026-02-19 14:32:02       | 22.5       | 22.4891    | 80.0
2026-02-19 14:32:03       | 22.6       | 22.6203    | 80.0
...
```

## Virtual Serial Port Setup

For testing without physical hardware, use virtual serial port pairs:

### Windows
Use **com0com** or **Virtual Serial Port Driver**:
```
Create pair: COM4 ↔ COM5
```

### Linux
Use **socat**:
```bash
socat -d -d pty,raw,echo=0 pty,raw,echo=0
# Note the /dev/pts/X numbers created
```

### macOS
Use built-in pseudo-terminals or **socat**.

## Development

### Project Structure
```
watlow_simulator/
├── Cargo.toml          # Project configuration and dependencies
├── Cargo.lock          # Locked dependency versions
├── src/
│   ├── main.rs         # Simulator server implementation
│   └── bin/
│       └── client.rs   # Client logger implementation
└── target/             # Build output (generated)
```

### Dependencies

- **tokio**: Async runtime
- **tokio-serial**: Serial port communication
- **tokio-modbus**: Modbus RTU protocol implementation
- **clap**: Command-line argument parsing
- **futures**: Async utilities
- **anyhow**: Error handling
- **chrono**: Timestamp formatting

### Running in Development Mode

```bash
# Run simulator
cargo run -- --port COM4 --baud 9600

# Run client
cargo run --bin watlow_simulator_client -- --port COM5 --baud 9600
```

## Troubleshooting

### Port Access Issues

**Windows:**
- Ensure no other application is using the COM port
- Check Device Manager for correct port number
- Try running with administrator privileges

**Linux:**
- Add user to `dialout` group: `sudo usermod -a -G dialout $USER`
- Check permissions: `ls -l /dev/ttyUSB0`
- May need to logout/login after group change

**macOS:**
- Check port list: `ls /dev/cu.*`
- Ensure user has permission to access serial devices

### Communication Errors

- Verify both server and client use the same baud rate
- Confirm correct serial port names
- Check that ports are correctly paired (for virtual ports)
- Ensure unit ID matches between client and server expectations
- Verify serial cable/adapter functionality (for hardware ports)

### Build Issues

```bash
# Clean and rebuild
cargo clean
cargo build --release
```

## Version History

- **0.1.0** - Initial release
  - Basic Watlow PM8 register support
  - Physics simulation
  - RTU server and client implementations


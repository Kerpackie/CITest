use clap::Parser;
use futures::future;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio_modbus::prelude::*;
use tokio_modbus::server::Service;
use tokio_serial::SerialPortBuilderExt;

/// WATLOW PM8 MODBUS RTU SIMULATOR
/// This version maps to official Watlow register addresses (360, 2322, 7101)
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Serial port (e.g., COM4, /dev/ttyUSB0, or virtual pts)
    #[arg(short, long)]
    port: String,

    /// Baud rate (Watlow default is 9600)
    #[arg(short, long, default_value_t = 9600)]
    baud: u32,
}

struct OvenState {
    pv: f32,
    sp: f32,
    ambient: f32,
}

struct WatlowService {
    state: Arc<Mutex<OvenState>>,
}

/// Helper to convert f32 to two u16s (IEEE 754) for Modbus Float Registers
fn f32_to_registers(val: f32) -> (u16, u16) {
    let bytes = val.to_bits();
    ((bytes >> 16) as u16, (bytes & 0xFFFF) as u16)
}

impl Service for WatlowService {
    type Request = Request<'static>;
    type Response = Response;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let mut oven = self.state.lock().unwrap();

        match req {
            // FC 03: Read Holding Registers
            Request::ReadHoldingRegisters(addr, cnt) => {
                let mut registers = vec![0u16; cnt as usize];

                for i in 0..cnt {
                    let reg_addr = addr + i;
                    registers[i as usize] = match reg_addr {
                        // --- Integer Registers (10x scaling) ---
                        // 100 is your original, 7101 is Watlow standard PV
                        100 | 7101 => (oven.pv * 10.0) as u16,

                        // 300 is active SP, 2322 is Setpoint 1
                        300 | 2322 => (oven.sp * 10.0) as u16,

                        // --- Floating Point Registers (IEEE 754) ---
                        // Reg 360 = high word, 361 = low word
                        360 => f32_to_registers(oven.pv).0,
                        361 => f32_to_registers(oven.pv).1,

                        _ => 0,
                    };
                }
                future::ready(Ok(Response::ReadHoldingRegisters(registers)))
            }

            // FC 06: Write Single Register
            Request::WriteSingleRegister(addr, val) => {
                if addr == 300 || addr == 2322 {
                    oven.sp = val as f32 / 10.0;
                    println!("SERVER: Setpoint changed to {:.1}°C", oven.sp);
                }
                future::ready(Ok(Response::WriteSingleRegister(addr, val)))
            }

            // FC 16: Write Multiple Registers
            Request::WriteMultipleRegisters(addr, values) => {
                if (addr == 300 || addr == 2322) && !values.is_empty() {
                    oven.sp = values[0] as f32 / 10.0;
                    println!("SERVER: Multi-write SP updated to {:.1}°C", oven.sp);
                }
                future::ready(Ok(Response::WriteMultipleRegisters(
                    addr,
                    values.len() as u16,
                )))
            }

            _ => future::ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Function code not supported by Simulator",
            ))),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // 1. Setup Shared State
    let state = Arc::new(Mutex::new(OvenState {
        pv: 22.1,
        sp: 50.0,
        ambient: 22.0,
    }));

    // 2. Physics Thread
    // Simulates thermal inertia: fast heating, slow cooling, ambient floor.
    let physics_state = state.clone();
    thread::spawn(move || {
        loop {
            {
                let mut oven = physics_state.lock().unwrap();
                let diff = oven.sp - oven.pv;

                if diff > 0.05 {
                    // Heating logic
                    oven.pv += 0.08;
                } else if oven.pv > oven.ambient {
                    // Passive cooling logic
                    oven.pv -= 0.02;
                }

                // Tiny noise floor for sensor realism
                let jitter = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_micros()
                    % 100) as f32
                    / 5000.0;
                oven.pv += jitter - 0.01;
            }
            thread::sleep(Duration::from_millis(200));
        }
    });

    println!("--- Watlow PM8 RTU Simulator ---");
    println!("Listening on: {} @ {} baud", args.port, args.baud);
    println!("Available Registers:");
    println!("  - Process Value (PV): 7101 (Int x10), 360 (32-bit Float)");
    println!("  - Setpoint (SP):     2322 (Int x10), 300 (Int x10)");

    // 3. Serial Port Initialization
    let port = tokio_serial::new(args.port, args.baud)
        .data_bits(tokio_serial::DataBits::Eight)
        .stop_bits(tokio_serial::StopBits::One)
        .parity(tokio_serial::Parity::None)
        .open_native_async()?;

    // 4. Run Modbus Server
    let service = WatlowService {
        state: state.clone(),
    };
    let server = tokio_modbus::server::rtu::Server::new(port);

    server.serve_forever(service).await?;

    Ok(())
}

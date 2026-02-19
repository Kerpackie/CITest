use clap::Parser;
use tokio_serial::SerialPortBuilderExt;
use tokio_modbus::prelude::*;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Serial port to connect to (e.g., COM5, /dev/pts/2)
    #[arg(short, long)]
    port: String,

    /// Baud rate
    #[arg(short, long, default_value_t = 9600)]
    baud: u32,

    /// Modbus Unit ID (Slave Address)
    #[arg(short, long, default_value_t = 1)]
    unit_id: u8,

    /// Optional: Write a specific Setpoint (SP) at startup
    #[arg(long)]
    set_sp: Option<f32>,

    /// Polling interval in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("--- Watlow PM8 Modbus Client ---");
    println!("Connecting to: {} @ {} baud (Slave ID: {})", args.port, args.baud, args.unit_id);

    // 1. Open Serial Port
    let port = tokio_serial::new(&args.port, args.baud)
        .data_bits(tokio_serial::DataBits::Eight)
        .stop_bits(tokio_serial::StopBits::One)
        .parity(tokio_serial::Parity::None)
        .open_native_async()?;

    // 2. Attach Modbus RTU Client
    let slave = Slave(args.unit_id);
    let mut ctx = tokio_modbus::client::rtu::attach_slave(port, slave);

    // 3. (Optional) Write Setpoint if requested via CLI
    if let Some(target_sp) = args.set_sp {
        println!(">>> WRITING SETPOINT: {:.1}°C", target_sp);
        // Watlow Integer SP is at register 2322 (scaled x10)
        let sp_int = (target_sp * 10.0) as u16;

        match ctx.write_single_register(2322, sp_int).await {
            Ok(_) => println!(">>> Setpoint Write Success"),
            Err(e) => eprintln!(">>> Setpoint Write Failed: {}", e),
        }
    }

    // 4. Polling Loop
    println!("\nStarting Logger (Ctrl+C to stop)...");
    println!("{:<25} | {:<10} | {:<10} | {:<10}", "Timestamp", "PV (Int)", "PV (F32)", "SP (Read)");
    println!("{}", "-".repeat(65));

    loop {
        // We read registers individually to test the simulator's full range.
        // In a production app, you might group these if addresses were contiguous.

        // A. Read PV (Integer x10) at 7101
        let pv_int = match ctx.read_holding_registers(7101, 1).await {
            Ok(data) => Some(data[0] as f32 / 10.0),
            Err(e) => {
                eprintln!("Error reading Reg 7101: {}", e);
                None
            }
        };

        // B. Read PV (Float 32-bit) at 360 (High Word) & 361 (Low Word)
        let pv_float = match ctx.read_holding_registers(360, 2).await {
            Ok(data) => {
                // Recombine two u16s into one f32
                let high = data[0];
                let low = data[1];
                let bits = ((high as u32) << 16) | (low as u32);
                Some(f32::from_bits(bits))
            }
            Err(e) => {
                eprintln!("Error reading Reg 360: {}", e);
                None
            }
        };

        // C. Read Active Setpoint (Integer x10) at 2322
        let sp_read = match ctx.read_holding_registers(2322, 1).await {
            Ok(data) => Some(data[0] as f32 / 10.0),
            Err(e) => {
                eprintln!("Error reading Reg 2322: {}", e);
                None
            }
        };

        // Log the line
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

        println!(
            "{:<25} | {:<10} | {:<10} | {:<10}",
            now,
            pv_int.map_or("ERR".to_string(), |v| format!("{:.1}", v)),
            pv_float.map_or("ERR".to_string(), |v| format!("{:.4}", v)),
            sp_read.map_or("ERR".to_string(), |v| format!("{:.1}", v))
        );

        tokio::time::sleep(Duration::from_millis(args.interval)).await;
    }
}
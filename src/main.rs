use std::{error, process::exit};

use bluest::{Adapter, AdvertisingDevice};
use clap::Parser;
use futures_util::StreamExt;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let adapter = Adapter::default().await.unwrap();
    adapter.wait_available().await?;

    println!("Starting scan");

    let discover_result = get_peripheral_with_name(args.name, &adapter).await?;
    let device = match discover_result {
        Some(dev) => dev.device,
        None => {
            eprintln!("No device found!");

            exit(-1);
        }
    };

    adapter.connect_device(&device).await?;
    println!("Connected to Device!");

    let services = device.discover_services().await?;

    let nus = services
        .iter()
        .filter(|service| {
            service
                .uuid()
                .to_string()
                .eq("6e400001-b5a3-f393-e0a9-e50e24dcca9e")
        })
        .nth(0)
        .unwrap();

    let characteristics = nus.discover_characteristics().await?;

    let nus_rx = characteristics
        .iter()
        .filter(|chara| {
            chara
                .uuid()
                .to_string()
                .eq("6e400002-b5a3-f393-e0a9-e50e24dcca9e")
        })
        .nth(0)
        .unwrap()
        .clone();

    let nus_tx = characteristics
        .iter()
        .filter(|chara| {
            chara
                .uuid()
                .to_string()
                .eq("6e400003-b5a3-f393-e0a9-e50e24dcca9e")
        })
        .nth(0)
        .unwrap();

    tokio::spawn(async move {
        loop {
            let payload = String::from("Hello from Rust!");
            println!("Write: {}", payload);

            nus_rx.write_without_response(payload.as_bytes()).await;

            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });

    let mut notify_stream = nus_tx.notify().await?;

    while let Some(stream) = notify_stream.next().await {
        if let Ok(read_from) = stream {
            println!("Read: {}", String::from_utf8(read_from).unwrap());
        }
    }

    adapter.disconnect_device(&device).await?;
    println!("Disconnected from device!");

    Ok(())
}

async fn get_peripheral_with_name(
    name: String,
    adapter: &Adapter,
) -> Result<Option<AdvertisingDevice>, Box<dyn error::Error>> {
    let mut scan = adapter.scan(&[]).await?;
    while let Some(discovered_device) = scan.next().await {
        let device_name = match discovered_device.device.name() {
            Ok(name) => name,
            Err(_) => String::from("<Name N/A>"),
        };

        if name == device_name {
            return Ok(Some(discovered_device));
        }
    }

    Ok(None)
}

use gilo::{HidDevice, HidDeviceInfo};
use tokio::{
    select,
    signal::ctrl_c,
    sync::broadcast::{self, Receiver},
    task::JoinSet,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, _) = broadcast::channel(1);
    let device_infos = gilo::scan_devices()?.filter(is_logitech_device);
    let mut join_set = JoinSet::new();
    for device_info in device_infos {
        let Ok(device) = HidDevice::new(device_info).await else {
            continue;
        };

        let rx = tx.subscribe();
        join_set.spawn(process_messages(device, rx));
    }

    select! {
        _ = join_set.join_all() => {},
        _ = ctrl_c() => {
            _ = tx.send(());
        }
    }

    Ok(())
}

fn is_logitech_device(device_info: &HidDeviceInfo) -> bool {
    device_info.vendor == 0x046d
}

async fn process_messages(mut device: HidDevice, cancel: Receiver<()>) {
    while cancel.is_empty() {
        match device.read().await {
            Ok(msg) => {
                let path = device.info.path.display();
                println!("Path: {}; Message: {:X?}", path, msg.as_bytes());
                continue;
            }
            Err(e) => {
                let path = device.info.path.display();
                eprintln!("Error reading from {path}: {e}");
                break;
            }
        }
    }
}

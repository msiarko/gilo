use gilo::{HidDevice, HidDeviceInfo, IntoHidDevice};
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
        let hid_device_result = device_info.into_hid_device().await;
        match hid_device_result {
            Ok(device) => {
                let rx = tx.subscribe();
                join_set.spawn(process_messages(device, rx));
            }
            Err(_) => continue,
        }
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
                let path = device.path().display();
                println!("Path: {}; Message: {:X?}", path, msg.as_bytes());
                continue;
            }
            Err(e) => {
                let path = device.path().display();
                eprintln!("Error reading from {path}: {e}");
                break;
            }
        }
    }
}

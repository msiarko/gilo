use gilo::{HidDevice, HidDeviceInfo, IntoHidDevice};
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let device_infos = gilo::scan_devices()?.filter(is_logitech_device);
    let mut join_set = JoinSet::new();
    for device_info in device_infos {
        let hid_device_result = device_info.into_hid_device().await;
        match hid_device_result {
            Ok(device) => {
                join_set.spawn(process_messages(device));
            }
            Err(err) => eprintln!("{err}"),
        }
    }

    join_set.join_all().await;
    Ok(())
}

fn is_logitech_device(device_info: &HidDeviceInfo) -> bool {
    device_info.vendor == 0x046d
}

async fn process_messages(mut device: HidDevice) {
    loop {
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

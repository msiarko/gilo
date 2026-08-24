mod hidapi;
mod hidpp;

pub fn start() -> anyhow::Result<()> {
    for device in hidapi::scan_devices()?.filter(|d| d.vendor == 0x046d) {
        println!(
            "Path: {}, VID: {:#04X}, PID: {:#04X}",
            device.path.display(),
            device.vendor,
            device.product
        );
    }

    Ok(())
}

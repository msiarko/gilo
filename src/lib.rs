use std::io;
mod hidapi;

pub fn start() -> io::Result<()> {
    for device in hidapi::scan_devices(0x046d)? {
        println!(
            "Path: {}, VID: {:#04X}, PID: {:#04X}",
            device.path.display(),
            device.vendor,
            device.product
        );
    }

    Ok(())
}

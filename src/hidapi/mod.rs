#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

use std::path::PathBuf;

pub struct HidDeviceInfo {
    pub path: PathBuf,
    pub vendor: u16,
    pub product: u16,
}

impl HidDeviceInfo {
    #[allow(dead_code)]
    fn new(path: PathBuf, vendor: u16, product: u16) -> Self {
        Self {
            path,
            vendor,
            product,
        }
    }
}

pub fn scan_devices() -> anyhow::Result<impl Iterator<Item = HidDeviceInfo>> {
    cfg_select! {
        target_os = "linux" => linux::scan_devices(),
        target_os = "windows" => windows::scan_devices(),
        _ => compile_error!("Unsupported platform"),
    }
}

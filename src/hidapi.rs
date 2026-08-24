use std::{
    fs::{self, OpenOptions},
    io,
    os::{fd::AsRawFd, unix::fs::FileTypeExt},
    path::PathBuf,
};

#[repr(C)]
#[derive(Default)]
struct HidRawDevInfo {
    bustype: u32,
    vendor: i16,
    product: i16,
}

const HIDIOCGRAWINFO: u64 = libc::_IOR::<HidRawDevInfo>('H' as u32, 0x03) as u64;

#[allow(dead_code)]
pub struct HidDeviceInfo {
    pub path: PathBuf,
    pub vendor: u16,
    pub product: u16,
}

impl HidDeviceInfo {
    fn new(path: PathBuf, vendor: u16, product: u16) -> Self {
        Self {
            path,
            vendor,
            product,
        }
    }
}

pub fn scan_devices(vendor_id: u16) -> io::Result<impl Iterator<Item = HidDeviceInfo>> {
    Ok(fs::read_dir("/dev")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_ok_and(|e| e.is_char_device())
                && entry.file_name().to_string_lossy().contains("hidraw")
        })
        .filter_map(move |entry| {
            let file = OpenOptions::new().read(true).open(entry.path()).ok()?;
            let mut dev_info = HidRawDevInfo::default();
            let rc = unsafe { libc::ioctl(file.as_raw_fd(), HIDIOCGRAWINFO, &mut dev_info) };
            if rc < 0 {
                return None;
            }

            let vid = i16::cast_unsigned(dev_info.vendor);
            let pid = i16::cast_unsigned(dev_info.product);

            if vid != vendor_id {
                return None;
            }

            Some(HidDeviceInfo::new(entry.path(), vid, pid))
        }))
}

#![allow(dead_code)]

use anyhow::anyhow;

use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::hidapi::HidDeviceInfo;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct HidMessageHeader {
    report_id: u8,
    device_index: u8,
    feature_index: u8,
    function_and_software_id: u8,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct HidShortMessage {
    header: HidMessageHeader,
    params: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct HidLongMessage {
    header: HidMessageHeader,
    params: [u8; 16],
}

#[derive(Debug, Clone)]
pub enum HidMessage {
    Short(HidShortMessage),
    Long(HidLongMessage),
    Unknown(Vec<u8>),
}

pub trait AsBytes: Sized {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

pub trait FromBytes: Sized + Copy {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let size = std::mem::size_of::<Self>();
        if bytes.len() < size {
            return Err(anyhow!(
                "Bytes length ({}) is less than message length ({})",
                bytes.len(),
                size
            ));
        }
        let align = std::mem::align_of::<Self>();
        let ptr = bytes.as_ptr();
        if ptr.align_offset(align) != 0 {
            return Err(anyhow!("Bytes slice and struct have different alignment"));
        }

        Ok(unsafe { *(ptr as *const Self) })
    }
}

impl AsBytes for HidShortMessage {}
impl FromBytes for HidShortMessage {}

impl AsBytes for HidLongMessage {}
impl FromBytes for HidLongMessage {}

impl HidMessage {
    pub fn from_bytes(value: &[u8]) -> anyhow::Result<Self> {
        if value.is_empty() {
            return Err(anyhow!("Empty message"));
        }

        match value[0] {
            0x10 | 0x02 => match HidShortMessage::from_bytes(value) {
                Ok(msg) => Ok(HidMessage::Short(msg)),
                Err(_) => Ok(HidMessage::Unknown(value.to_vec())),
            },
            0x11 | 0x03 => match HidLongMessage::from_bytes(value) {
                Ok(msg) => Ok(HidMessage::Long(msg)),
                Err(_) => Ok(HidMessage::Unknown(value.to_vec())),
            },
            _ => Ok(HidMessage::Unknown(value.to_vec())),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            HidMessage::Short(short) => short.as_bytes(),
            HidMessage::Long(long) => long.as_bytes(),
            HidMessage::Unknown(raw) => raw.as_slice(),
        }
    }
}

pub struct HidDevice {
    pub info: HidDeviceInfo,
    file: fs::File,
    buf: [u8; std::mem::size_of::<HidLongMessage>()],
}

impl HidDevice {
    pub async fn new(info: HidDeviceInfo) -> anyhow::Result<HidDevice> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&info.path)
            .await
            .map_err(|err| anyhow!("Path: {}; Error: {}", info.path.display(), err))?;
        Ok(HidDevice {
            info: info,
            file,
            buf: [0; std::mem::size_of::<HidLongMessage>()],
        })
    }

    pub async fn read(&mut self) -> anyhow::Result<HidMessage> {
        let bytes_read = self.file.read(&mut self.buf).await?;
        HidMessage::from_bytes(&self.buf[..bytes_read])
    }

    pub async fn write(&mut self, message: &HidMessage) -> anyhow::Result<()> {
        let bytes = message.as_bytes();
        let written = self.file.write(bytes).await?;
        assert_eq!(bytes.len(), written);
        Ok(())
    }
}

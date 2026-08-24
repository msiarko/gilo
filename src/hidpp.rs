#![allow(dead_code)]

use anyhow::anyhow;

use crate::hidapi::HidDeviceInfo;
use std::{
    fs,
    io::{Read, Write},
};

#[repr(C)]
#[derive(Debug, Default)]
struct HidMessageHeader {
    report_id: u8,
    device_index: u8,
    feature_index: u8,
    function_and_software_id: u8,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct HidShortMessage {
    header: HidMessageHeader,
    params: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct HidLongMessage {
    header: HidMessageHeader,
    params: [u8; 16],
}

#[derive(Debug)]
pub enum HidMessage<'a> {
    Short(&'a HidShortMessage),
    Long(&'a HidLongMessage),
}

trait AsBytes: Sized {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

trait FromBytes: Sized {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<&Self> {
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

        Ok(unsafe { &*(ptr as *const Self) })
    }
}

impl AsBytes for HidShortMessage {}
impl FromBytes for HidShortMessage {}

impl AsBytes for HidLongMessage {}
impl FromBytes for HidLongMessage {}

impl<'a> HidMessage<'a> {
    fn from_bytes(value: &'a [u8]) -> anyhow::Result<Self> {
        if value.is_empty() {
            return Err(anyhow!("Empty message"));
        }

        match value[0] {
            0x10 | 0x02 => Ok(HidMessage::Short(HidShortMessage::from_bytes(value)?)),
            0x11 | 0x03 => Ok(HidMessage::Long(HidLongMessage::from_bytes(value)?)),
            _ => Err(anyhow!("Unknown report ID")),
        }
    }

    fn as_bytes(&self) -> &[u8] {
        match self {
            HidMessage::Short(short) => short.as_bytes(),
            HidMessage::Long(long) => long.as_bytes(),
        }
    }
}

pub struct HidDevice {
    file: fs::File,
    buf: [u8; std::mem::size_of::<HidLongMessage>()],
}

impl HidDevice {
    pub fn read(&mut self) -> anyhow::Result<HidMessage<'_>> {
        let bytes_read = self.file.read(&mut self.buf)?;
        HidMessage::from_bytes(&self.buf[..bytes_read])
    }

    pub fn write(&mut self, message: &HidMessage) -> anyhow::Result<()> {
        let bytes = message.as_bytes();
        let written = self.file.write(bytes)?;
        assert_eq!(bytes.len(), written);
        Ok(())
    }
}

pub trait IntoHidDevice {
    fn open(&self) -> anyhow::Result<HidDevice>;
}

impl IntoHidDevice for HidDeviceInfo {
    fn open(&self) -> anyhow::Result<HidDevice> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)?;
        Ok(HidDevice {
            file,
            buf: [0; std::mem::size_of::<HidLongMessage>()],
        })
    }
}

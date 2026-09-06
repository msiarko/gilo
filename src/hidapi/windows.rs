use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use anyhow::bail;
use windows_sys::Win32::{
    CloseHandle, CreateFileW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, FILE_SHARE_READ,
    FILE_SHARE_WRITE, HANDLE, HIDD_ATTRIBUTES, HidD_GetAttributes, HidD_GetHidGuid,
    INVALID_HANDLE_VALUE, OPEN_EXISTING, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
};
use windows_sys::core::GUID;

use crate::hidapi::HidDeviceInfo;

pub(super) fn scan_devices() -> anyhow::Result<impl Iterator<Item = HidDeviceInfo>> {
    WinHidDeviceIterator::new()
}

struct DeviceInfoSet(HANDLE);

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                SetupDiDestroyDeviceInfoList(self.0);
            }
        }
    }
}

struct SafeHandle(HANDLE);

impl Drop for SafeHandle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

struct WinHidDeviceIterator {
    device_info_set: DeviceInfoSet,
    guid: GUID,
    index: u32,
}

enum DeviceError {
    NoMoreItems,
    Skip,
}

impl WinHidDeviceIterator {
    fn new() -> anyhow::Result<Self> {
        let mut guid = GUID::default();
        let hdev = unsafe {
            HidD_GetHidGuid(&mut guid);
            SetupDiGetClassDevsW(
                &guid,
                std::ptr::null(),
                std::ptr::null_mut(),
                (DIGCF_PRESENT | DIGCF_DEVICEINTERFACE).cast_unsigned(),
            )
        };

        if hdev == INVALID_HANDLE_VALUE {
            bail!("Failed to get device info set");
        }

        Ok(Self {
            device_info_set: DeviceInfoSet(hdev),
            guid,
            index: 0,
        })
    }

    fn try_get_device(&self, index: u32) -> Result<HidDeviceInfo, DeviceError> {
        let mut iface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };

        let success = unsafe {
            SetupDiEnumDeviceInterfaces(
                self.device_info_set.0,
                std::ptr::null(),
                &self.guid,
                index,
                &mut iface_data,
            )
        };

        if success == 0 {
            return Err(DeviceError::NoMoreItems);
        }

        let mut required_size: u32 = 0;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                self.device_info_set.0,
                &iface_data,
                std::ptr::null_mut(),
                0,
                &mut required_size,
                std::ptr::null_mut(),
            )
        };

        if required_size == 0 {
            return Err(DeviceError::Skip);
        }

        // Allocate buffer for SP_DEVICE_INTERFACE_DETAIL_DATA_W including the variable length string
        let mut detail_data_buf = vec![0u16; (required_size as usize).div_ceil(2)];
        let detail_data = detail_data_buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;

        unsafe {
            (*detail_data).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        }

        let success = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                self.device_info_set.0,
                &iface_data,
                detail_data,
                required_size,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        if success == 0 {
            return Err(DeviceError::Skip);
        }

        let raw_handle = unsafe {
            CreateFileW(
                (*detail_data).DevicePath.as_ptr(),
                0,
                (FILE_SHARE_READ | FILE_SHARE_WRITE).cast_unsigned(),
                std::ptr::null_mut(),
                OPEN_EXISTING.cast_unsigned(),
                0,
                std::ptr::null_mut(),
            )
        };

        if raw_handle == INVALID_HANDLE_VALUE {
            return Err(DeviceError::Skip);
        }

        let handle = SafeHandle(raw_handle);
        let mut attrs = HIDD_ATTRIBUTES::default();
        if !unsafe { HidD_GetAttributes(handle.0, &mut attrs) } {
            return Err(DeviceError::Skip);
        }

        let path = unsafe {
            let mut path_len = 0;
            let device_path_ptr = (*detail_data).DevicePath.as_ptr();
            while *device_path_ptr.add(path_len) != 0 {
                path_len += 1;
            }
            let slice = std::slice::from_raw_parts(device_path_ptr, path_len);
            OsString::from_wide(slice)
        };

        Ok(HidDeviceInfo::new(
            PathBuf::from(path),
            attrs.VendorID,
            attrs.ProductID,
        ))
    }
}

impl Iterator for WinHidDeviceIterator {
    type Item = HidDeviceInfo;

    fn next(&mut self) -> Option<HidDeviceInfo> {
        loop {
            match self.try_get_device(self.index) {
                Ok(device) => {
                    self.index += 1;
                    return Some(device);
                }
                Err(DeviceError::Skip) => {
                    self.index += 1;
                    continue;
                }
                Err(DeviceError::NoMoreItems) => {
                    return None;
                }
            }
        }
    }
}

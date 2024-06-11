use std::ptr;

use windows::{
    core::{Result, HRESULT, PWSTR},
    Win32::{
        Devices::Properties,
        Media::Audio::IMMDevice,
        System::{
            Com::{StructuredStorage, STGM_READ},
            Variant::{VT_EMPTY, VT_LPWSTR},
        },
        UI::Shell::PropertiesSystem::PROPERTYKEY,
    },
};

static FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: Properties::DEVPKEY_Device_FriendlyName.fmtid,
    pid: Properties::DEVPKEY_Device_FriendlyName.pid,
};

#[allow(overflowing_literals)]
pub static ELEMENT_NOT_FOUND: i32 = 0x8002_802B_i32;
#[allow(overflowing_literals)]
pub static BAD_VALUE: i32 = 0x8000_1054_i32;

pub fn get_device_name(device: &IMMDevice) -> Result<String> {
    unsafe {
        let property_store = device.OpenPropertyStore(STGM_READ)?;
        let Ok(mut name_prop_variant) = property_store.GetValue(ptr::addr_of!(FRIENDLY_NAME))
        else {
            return Ok("Unknown".to_string());
        };
        let prop_variant_inner = &name_prop_variant.as_raw().Anonymous.Anonymous;
        if prop_variant_inner.vt == VT_EMPTY.0 {
            return Err(windows::core::Error::new(
                HRESULT(ELEMENT_NOT_FOUND),
                "Empty property",
            ));
        }
        if prop_variant_inner.vt != VT_LPWSTR.0 {
            return Err(windows::core::Error::new(
                HRESULT(BAD_VALUE),
                "Unexpected property type",
            ));
        }
        let inner = prop_variant_inner.Anonymous.pwszVal;
        let name = PWSTR(inner);
        let name_string = name.to_string()?;

        StructuredStorage::PropVariantClear(&mut name_prop_variant)?;
        Ok(name_string)
    }
}

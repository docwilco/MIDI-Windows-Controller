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
                HRESULT(0x8002_802B_u32 as i32),
                "Empty property",
            ));
        }
        assert_eq!(prop_variant_inner.vt, VT_LPWSTR.0);

        let name_ptr = ptr::addr_of!(prop_variant_inner.Anonymous);
        let name = PWSTR(name_ptr as *mut _);
        let name_string = name.to_string()?;

        StructuredStorage::PropVariantClear(&mut name_prop_variant)?;
        Ok(name_string)
    }
}

use std::{
    collections::HashSet, ffi::OsString, os::windows::ffi::OsStringExt, slice, thread::sleep,
    time::Duration,
};

use static_assertions::const_assert_eq;
use sysinfo::{Pid, System};

use windows::{
    core::{Error, Interface},
    Win32::{
        Devices::Properties,
        Foundation::TRUE,
        Media::Audio::{
            eConsole, eRender, Endpoints::IAudioEndpointVolume, IAudioSessionControl,
            IAudioSessionControl2, IAudioSessionEnumerator, IAudioSessionManager2, IMMDevice,
            IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, StructuredStorage, CLSCTX_ALL,
            COINIT_APARTMENTTHREADED, STGM_READ,
        },
    },
};

use windows::Win32::{
    System::Variant::VT_LPWSTR,
    UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};

fn get_device_name(device: &IMMDevice) -> Result<String, Error> {
    unsafe {
        let property_store = device.OpenPropertyStore(STGM_READ).unwrap();
        let mut name_prop_variant = property_store
            .GetValue(&Properties::DEVPKEY_Device_FriendlyName as *const _ as *const _)
            .unwrap();
        let prop_variant_inner = &name_prop_variant.as_raw().Anonymous.Anonymous;
        assert_eq!(prop_variant_inner.vt, VT_LPWSTR.0);
        let ptr_utf16 = *(&prop_variant_inner.Anonymous as *const _ as *const *const u16);

        // Find the length of the friendly name.
        let mut len = 0;
        while *ptr_utf16.offset(len) != 0 {
            len += 1;
        }

        // Create the utf16 slice and convert it into a string.
        let name_slice = slice::from_raw_parts(ptr_utf16, len as usize);
        let name_os_string: OsString = OsStringExt::from_wide(name_slice);
        let name_string = match name_os_string.into_string() {
            Ok(string) => string,
            Err(os_string) => os_string.to_string_lossy().into(),
        };

        // Clean up the property.
        StructuredStorage::PropVariantClear(&mut name_prop_variant).ok();
        Ok(name_string)
    }
}

fn main() -> Result<(), Error> {
    const_assert_eq!('{'.len_utf16(), 1);
    const_assert_eq!('}'.len_utf16(), 1);
    let mut openbracket: [u16; 1] = [0];
    let mut closebracket: [u16; 1] = [0];
    '{'.encode_utf16(&mut openbracket);
    '}'.encode_utf16(&mut closebracket);
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;

        // Create an instance of MMDeviceEnumerator
        let enumerator =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .unwrap();

        let default_device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .unwrap();
        let session_manager2 =
            default_device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)?;
        let mut system = System::new_all();

        loop {
            let default_device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .unwrap();
            let name_string = get_device_name(&default_device)?;
            let default_volume =
                default_device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)?;
            println!(
                "Default device: {} ({:.2})",
                name_string,
                default_volume.GetMasterVolumeLevelScalar()?
            );
            let session_collection = session_manager2.GetSessionEnumerator()?;
            let foreground = GetForegroundWindow();
            let mut window_pid: u32 = 0;
            let _ = GetWindowThreadProcessId(foreground, Some(&mut window_pid as *mut _));
            system.refresh_processes();
            let session_pids = session_pids(&session_collection)?;
            let process = system.process(Pid::from_u32(window_pid));

            if let Some(proc) = process {
                let pids = pid_and_child_pids(proc.pid(), &system);
                let intersection = session_pids
                    .intersection(&pids)
                    .copied()
                    .collect::<Vec<_>>();
                for pid in intersection {
                    if let Some(session) = get_session_for_pid(pid.as_u32(), &session_collection)? {
                        let volume_control = session.cast::<ISimpleAudioVolume>()?;
                        print!("{}: ", proc.name());
                        if volume_control.GetMute()? == TRUE {
                            println!("Muted");
                        } else {
                            println!("{:.2}", volume_control.GetMasterVolume()?);
                        }
                    }
                }
            }
            sleep(Duration::from_secs(1));
        }
    }
}

fn get_session_for_pid(
    pid: u32,
    session_collection: &IAudioSessionEnumerator,
) -> Result<Option<IAudioSessionControl>, Error> {
    for i in 0..unsafe { session_collection.GetCount() }? {
        let session = unsafe { session_collection.GetSession(i) }?;
        let session_ext: IAudioSessionControl2 = session.cast::<IAudioSessionControl2>()?;
        let session_pid = unsafe { session_ext.GetProcessId() }?;
        if session_pid == pid {
            return Ok(Some(session));
        }
    }
    Ok(None)
}

fn session_pids(session_collection: &IAudioSessionEnumerator) -> Result<HashSet<Pid>, Error> {
    Ok((0..unsafe { session_collection.GetCount() }?)
        .map(|i| unsafe { session_collection.GetSession(i) })
        .filter_map(|session| {
            session.ok().and_then(|session| {
                let session_ext: IAudioSessionControl2 =
                    session.cast::<IAudioSessionControl2>().ok()?;
                let pid = unsafe { session_ext.GetProcessId() }.ok()?;
                Some(Pid::from_u32(pid))
            })
        })
        .collect())
}

fn pid_and_child_pids(parent_pid: Pid, system: &System) -> HashSet<Pid> {
    let mut children = vec![HashSet::from([parent_pid])];
    loop {
        let new_children = system
            .processes()
            .iter()
            .filter_map(|(pid, proc)| {
                if proc
                    .parent()
                    .map_or(false, |parent| children.last().unwrap().contains(&parent))
                {
                    Some(*pid)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        if new_children.is_empty() {
            break;
        }
        children.push(new_children);
    }
    children.into_iter().flatten().collect()
}

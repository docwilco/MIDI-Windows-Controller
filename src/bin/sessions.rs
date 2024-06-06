use std::{thread::sleep, time::Duration};

use static_assertions::const_assert_eq;
use sysinfo::{Pid, System};

use windows::{
    core::{Error, Interface},
    Win32::{
        Foundation::{S_FALSE, S_OK},
        Media::Audio::{
            eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, 
            IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_ALL,
            COINIT_APARTMENTTHREADED, 
        },
    },
};

#[path = "../utils.rs"]
mod utils;
use utils::get_device_name;

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
        let name_string = get_device_name(&default_device)?;
        println!("Default device: {:?}", name_string);

        let session_manager = default_device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)?;
        let mut system = System::new_all();
        loop {
            println!("==========================================");
            let session_collection = session_manager.GetSessionEnumerator()?;
            system.refresh_processes();
            for i in 0..session_collection.GetCount()? {
                let session = session_collection.GetSession(i)?;
                let session_state = session.GetState()?;
                let display_name = session.GetDisplayName()?;
                let grouping_param = session.GetGroupingParam()?;
                let icon_path = session.GetIconPath()?;
                let session_ext: IAudioSessionControl2 = session.cast::<IAudioSessionControl2>()?;
                let session_vol = session_ext.cast::<ISimpleAudioVolume>()?;
                let pid = session_ext.GetProcessId()?;
                let is_system_sounds_session = match session_ext.IsSystemSoundsSession() {
                    r if r == S_OK => Ok(true),
                    r if r == S_FALSE => Ok(false),
                    e => Err(e),
                }?;

                //                let session_id = session_ext.GetSessionIdentifier()?;
                //                let volume_control =
                //                    get_volume_control(session_id, openbracket, closebracket, &session_manager)?;
                //                let instance_volume_control = get_volume_control(
                //                    session_ext.GetSessionInstanceIdentifier()?,
                //                    openbracket,
                //                    closebracket,
                //                    &session_manager,
                //                )?;
                println!(
                    "{}: {:?} [{:?}] {}",
                    display_name.to_string()?,
                    session_state,
                    grouping_param,
                    icon_path.to_string()?
                );
                if is_system_sounds_session {
                    println!("  System sounds session");
                }
                //println!("  pid={} sid={:?}", pid, session_id.to_string()?);
                if let Some(process) = system.process(Pid::from_u32(pid)) {
                    println!("  pname={}", process.name());
                }
                println!(
                    "  Volume: {:.2} Mute: {:?}",
                    session_vol.GetMasterVolume()?,
                    session_vol.GetMute()?,
                );
            }
            sleep(Duration::from_secs(1));
        }
    }
}

//unsafe fn get_volume_control(
//    session_id: windows::core::PWSTR,
//    openbracket: [u16; 1],
//    closebracket: [u16; 1],
//    session_manager: &IAudioSessionManager2,
//) -> Result<windows::Win32::Media::Audio::ISimpleAudioVolume, Error> {
//    let mut session_identifier = session_id.as_wide().to_vec();
//    let mut id_iter = session_identifier.iter_mut();
//    assert_eq!(id_iter.position(|c| *c == openbracket[0]), Some(0));
//    let guid_start = id_iter.position(|c| *c == openbracket[0]).unwrap() + 1;
//    let guid_end = id_iter.position(|c| *c == closebracket[0]).unwrap() + guid_start + 1;
//    session_identifier[guid_end + 1] = 0;
//    let guid = &mut session_identifier[guid_start..=guid_end];
//    let cguid = PCWSTR(guid.as_ptr() as _);
//    let mut guid = CLSIDFromString(cguid)?;
//    let volume_control =
//        session_manager.GetSimpleAudioVolume(Some(&mut guid as *mut _), TRUE.0 as u32)?;
//    Ok(volume_control)
//}

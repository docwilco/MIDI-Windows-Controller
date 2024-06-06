use sysinfo::{Pid, System};

use windows::{
    core::{Error, Interface},
    Win32::{
        Foundation::{S_FALSE, S_OK},
        Media::Audio::{
            eConsole, eRender, Endpoints::IAudioEndpointVolume, IAudioSessionControl2,
            IAudioSessionManager2, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED},
    },
};

#[path = "../utils.rs"]
mod utils;
use utils::get_device_name;

fn main() -> Result<(), Error> {
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
        println!("Default device: {name_string:?}");
        let endpoints = enumerator
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .unwrap();
        for i in 0..endpoints.GetCount().unwrap() {
            let device = endpoints.Item(i).unwrap();
            let name_string = get_device_name(&device)?;
            println!("Device {i}: {name_string:?}");
        }

        let default_volume = default_device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)?;
        println!("{:?}", default_volume.GetMasterVolumeLevelScalar());

        let session_manager = default_device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)?;
        let session_collection = session_manager.GetSessionEnumerator()?;
        let mut system = System::new_all();
        system.refresh_processes();
        for i in 0..session_collection.GetCount()? {
            let session = session_collection.GetSession(i)?;
            let session_state = session.GetState()?;
            let display_name = session.GetDisplayName()?;
            let grouping_param = session.GetGroupingParam()?;
            let icon_path = session.GetIconPath()?;
            let session_ext: IAudioSessionControl2 = session.cast::<IAudioSessionControl2>()?;
            let pid = session_ext.GetProcessId()?;
            let is_system_sounds_session = match session_ext.IsSystemSoundsSession() {
                r if r == S_OK => Ok(true),
                r if r == S_FALSE => Ok(false),
                e => Err(e),
            }?;
            let session_id = session_ext.GetSessionIdentifier()?;

            println!(
                "{}: {:?} [{:?}] {}",
                display_name.to_string()?,
                session_state,
                grouping_param,
                icon_path.to_string()?
            );
            println!("  {:?} {:?}", pid, is_system_sounds_session);
            println!("  {:?}", session_id.to_string()?);
            if let Some(process) = system.process(Pid::from_u32(pid)) {
                println!("  {:?}", process.name());
            }
        }
    }
    Ok(())
}

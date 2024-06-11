use static_assertions::const_assert_eq;
use std::{collections::HashSet, ptr, thread::sleep, time::Duration};
use sysinfo::{Pid, System};
use windows::{
    core::{Error, Interface},
    Win32::{
        Foundation::TRUE,
        Media::Audio::{
            eConsole, eRender, Endpoints::IAudioEndpointVolume, IAudioSessionControl,
            IAudioSessionControl2, IAudioSessionEnumerator, IAudioSessionManager2,
            IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED},
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
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
            let _ = GetWindowThreadProcessId(foreground, Some(ptr::addr_of_mut!(window_pid)));
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
                let session_ext = session.cast::<IAudioSessionControl2>().ok()?;
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

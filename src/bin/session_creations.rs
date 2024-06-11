// Register an IAudioSessionNotification on every device and print the session creation events.
use std::rc::Rc;
use windows::{
    core::{implement, Result},
    Win32::{
        Media::Audio::{
            eAll, IAudioSessionControl, IAudioSessionManager2, IAudioSessionNotification,
            IAudioSessionNotification_Impl, IMMDeviceEnumerator, MMDeviceEnumerator,
            DEVICE_STATE_ACTIVE,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED},
    },
};

#[path = "../utils.rs"]
mod utils;
use utils::get_device_name;

#[implement(IAudioSessionNotification)]
struct AudioSessionNotification {
    device_id_string: String,
    device_name: String,
    //audio_session_manager: Arc<IAudioSessionManager2>,
}

impl IAudioSessionNotification_Impl for AudioSessionNotification {
    fn OnSessionCreated(&self, session: Option<&IAudioSessionControl>) -> Result<()> {
        println!(
            "Session Created on {} ({})",
            self.device_name, self.device_id_string
        );
        let Some(session) = session else {
            return Ok(());
        };
        let display_name = unsafe { session.GetDisplayName() }?;
        let display_name = unsafe { display_name.to_string() }?;
        println!(
            "Session Created on {}: {}",
            self.device_id_string, display_name
        );
        Ok(())
    }
}

fn main() -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }?;

    // Create an instance of MMDeviceEnumerator
    let enumerator = unsafe {
        CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap()
    };
    let mut notifications = Vec::new();
    let mut session_managers = Vec::new();

    // Enumerate audio endpoints
    let devices = unsafe { enumerator.EnumAudioEndpoints(eAll, DEVICE_STATE_ACTIVE) }?;
    for i in 0..unsafe { devices.GetCount() }? {
        let device = unsafe { devices.Item(i) }?;
        let name = get_device_name(&device)?;
        let id = unsafe { device.GetId()?.to_string() }?;
        println!("Device: {name} ({id})");

        println!("Registering for audio session events on {name}...");
        let audio_session_manager =
            Rc::new(unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) }?);
        let notification = AudioSessionNotification {
            device_id_string: id,
            device_name: name.clone(),
            //audio_session_manager: audio_session_manager.clone(),
        };
        let sessionnotification = IAudioSessionNotification::from(notification);
        unsafe { audio_session_manager.RegisterSessionNotification(&sessionnotification) }?;
        notifications.push(sessionnotification);
        let session_collection = unsafe { audio_session_manager.GetSessionEnumerator() }?;
        for j in 0..unsafe { session_collection.GetCount() }? {
            let session = unsafe { session_collection.GetSession(j) }?;
            let display_name = unsafe { session.GetDisplayName() }?;
            let display_name = unsafe { display_name.to_string() }?;
            println!("Session: {display_name}");
        }
        session_managers.push(audio_session_manager);
        println!("Registered");
    }

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

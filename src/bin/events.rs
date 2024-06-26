#![allow(non_upper_case_globals)]
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{
        mpsc::{self, Sender},
        Mutex, OnceLock,
    },
    thread,
    time::Instant,
};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
use windows::{
    core::{implement, w, Error, Interface, Result, GUID, HRESULT, PCWSTR, PWSTR},
    Win32::{
        Foundation::{GetLastError, BOOL, HWND, S_OK, TRUE},
        Media::Audio::{
            eAll, eCapture, eCommunications, eConsole, eMultimedia, eRender,
            AudioSessionDisconnectReason, AudioSessionState, AudioSessionStateActive,
            AudioSessionStateExpired, AudioSessionStateInactive, DisconnectReasonDeviceRemoval,
            DisconnectReasonExclusiveModeOverride, DisconnectReasonFormatChanged,
            DisconnectReasonServerShutdown, DisconnectReasonSessionDisconnected,
            DisconnectReasonSessionLogoff, EDataFlow as WindowsEDataFlow, ERole as WindowsERole,
            IAudioSessionControl, IAudioSessionControl2, IAudioSessionEvents,
            IAudioSessionEvents_Impl, IAudioSessionManager2, IAudioSessionNotification,
            IAudioSessionNotification_Impl, IMMDevice, IMMDeviceEnumerator, IMMNotificationClient,
            IMMNotificationClient_Impl, MMDeviceEnumerator, DEVICE_STATE, DEVICE_STATE_ACTIVE,
            DEVICE_STATE_DISABLED, DEVICE_STATE_NOTPRESENT, DEVICE_STATE_UNPLUGGED,
        },
        System::{
            Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
            Diagnostics::Debug::{
                FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS,
            },
            SystemServices::LANG_NEUTRAL,
        },
        UI::{
            Accessibility::{SetWinEventHook, HWINEVENTHOOK},
            Shell::PropertiesSystem::PROPERTYKEY,
            WindowsAndMessaging::{
                CreateWindowExW, DestroyWindow, GetForegroundWindow, GetMessageW,
                GetWindowThreadProcessId, EVENT_SYSTEM_FOREGROUND, HWND_MESSAGE, MSG,
                WINDOW_EX_STYLE, WINDOW_STYLE, WINEVENT_OUTOFCONTEXT,
            },
        },
    },
};

#[path = "../utils.rs"]
mod utils;
use utils::{get_device_name, BAD_VALUE};

struct SessionSimpleVolumeChangedEvent {
    volume: f32,
    mute: bool,
}

enum SessionEvent {
    SimpleVolumeChanged(SessionSimpleVolumeChangedEvent),
    DisplayNameChanged(String),
    IconPathChanged(String),
    GroupingParamChanged(u128),
    StateChanged(SessionState),
    SessionDisconnected(DisconnectReason),
}

enum DeviceEvent {
    DefaultDeviceChanged(EDataFlow, ERole),
    DeviceAdded,
    DeviceRemoved,
    DeviceStateChanged(DEVICE_STATE),
    SessionCreated(String),
}

enum Event {
    Device(String, DeviceEvent),
    Session(String, String, SessionEvent),
    ActiveWindowChange(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::Display)]
enum EDataFlow {
    Render = 0,
    Capture = 1,
    All,
}

impl TryFrom<WindowsEDataFlow> for EDataFlow {
    type Error = Error;
    fn try_from(value: WindowsEDataFlow) -> Result<Self> {
        match value {
            eRender => Ok(EDataFlow::Render),
            eCapture => Ok(EDataFlow::Capture),
            eAll => Ok(EDataFlow::All),
            _ => Err(Error::new(HRESULT(BAD_VALUE), "Bad value for flow")),
        }
    }
}

impl From<EDataFlow> for WindowsEDataFlow {
    fn from(value: EDataFlow) -> WindowsEDataFlow {
        match value {
            EDataFlow::Render => eRender,
            EDataFlow::Capture => eCapture,
            EDataFlow::All => eAll,
        }
    }
}

#[derive(Clone, Copy, Debug, strum::Display)]
enum ERole {
    Console = 0,
    Multimedia = 1,
    Communications = 2,
}

impl TryFrom<WindowsERole> for ERole {
    type Error = Error;
    fn try_from(value: WindowsERole) -> Result<Self> {
        match value {
            eConsole => Ok(ERole::Console),
            eMultimedia => Ok(ERole::Multimedia),
            eCommunications => Ok(ERole::Communications),
            _ => Err(Error::new(HRESULT(BAD_VALUE), "Bad value for role")),
        }
    }
}

impl From<ERole> for WindowsERole {
    fn from(value: ERole) -> WindowsERole {
        match value {
            ERole::Console => eConsole,
            ERole::Multimedia => eMultimedia,
            ERole::Communications => eCommunications,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, strum::Display)]
enum SessionState {
    Active,
    Inactive,
    Expired,
}

impl TryFrom<AudioSessionState> for SessionState {
    type Error = Error;
    fn try_from(value: AudioSessionState) -> Result<Self> {
        match value {
            AudioSessionStateActive => Ok(SessionState::Active),
            AudioSessionStateInactive => Ok(SessionState::Inactive),
            AudioSessionStateExpired => Ok(SessionState::Expired),
            _ => Err(Error::new(HRESULT(BAD_VALUE), "Bad value for state")),
        }
    }
}

#[derive(Clone, Debug, strum::Display)]
enum DisconnectReason {
    DeviceRemoval,
    ServerShutdown,
    FormatChanged,
    SessionLogoff,
    SessionDisconnected,
    ExclusiveModeOverride,
    SessionExpired,
}

impl TryFrom<AudioSessionDisconnectReason> for DisconnectReason {
    type Error = Error;
    fn try_from(reason: AudioSessionDisconnectReason) -> Result<Self> {
        Ok(match reason {
            DisconnectReasonDeviceRemoval => DisconnectReason::DeviceRemoval,
            DisconnectReasonServerShutdown => DisconnectReason::ServerShutdown,
            DisconnectReasonFormatChanged => DisconnectReason::FormatChanged,
            DisconnectReasonSessionLogoff => DisconnectReason::SessionLogoff,
            DisconnectReasonSessionDisconnected => DisconnectReason::SessionDisconnected,
            DisconnectReasonExclusiveModeOverride => DisconnectReason::ExclusiveModeOverride,
            _ => {
                return Err(Error::new(
                    HRESULT(BAD_VALUE),
                    "Bad value for disconnect reason",
                ))
            }
        })
    }
}

#[derive(Clone, Debug, strum::Display)]
enum DeviceState {
    Active(IAudioSessionManager2),
    Disabled,
    NotPresent,
    Unplugged,
}

struct SessionInfo {
    instance_id: String,
    _id: String,
    control: IAudioSessionControl,
    control2: IAudioSessionControl2,
    display_name: Option<String>,
    _state: SessionState,
    pid: u32,
    // We need to keep a reference to this to keep it alive
    #[allow(dead_code)]
    session_events: IAudioSessionEvents,
}

impl SessionInfo {
    fn new(
        device_info: &DeviceInfo,
        instance_id: String,
        control: IAudioSessionControl,
        control2: IAudioSessionControl2,
        event_tx: Sender<Event>,
    ) -> Result<Self> {
        let id = unsafe { control2.GetSessionIdentifier()?.to_string() }?;
        let session_events = IAudioSessionEvents::from(AudioSessionEvents {
            device_id: device_info.id.clone(),
            session_instance_id: instance_id.clone(),
            event_tx,
        });
        unsafe { control.RegisterAudioSessionNotification(&session_events) }?;
        let state = SessionState::try_from(unsafe { control.GetState()? })?;
        let pid = unsafe { control2.GetProcessId()? };
        let mut session_info = Self {
            instance_id,
            _id: id,
            control,
            control2,
            display_name: None,
            _state: state,
            pid,
            session_events,
        };
        session_info.set_display_name(Some(unsafe {
            session_info.control.GetDisplayName()?.to_string()
        }?))?;
        Ok(session_info)
    }
    fn set_display_name(&mut self, new_display_name: Option<String>) -> Result<()> {
        let new_display_name = match new_display_name.as_deref() {
            Some("") => None,
            _ => new_display_name,
        };
        self.display_name = if let Some(new_display_name) = new_display_name {
            Some(new_display_name)
        } else {
            let pid = unsafe { self.control2.GetProcessId()? };
            let system = System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new()),
            );
            system
                .process(Pid::from_u32(pid))
                .map(|process| process.name().to_string())
        };
        Ok(())
    }
}

impl Drop for SessionInfo {
    fn drop(&mut self) {
        unsafe {
            self.control
                .UnregisterAudioSessionNotification(&self.session_events)
        }
        .unwrap();
    }
}

struct DeviceInfo {
    device: IMMDevice,
    session_map: HashMap<String, SessionInfo>,
    id: String,
    name: String,
    state: DeviceState,
    event_tx: Sender<Event>,
}

impl DeviceInfo {
    fn new(device: IMMDevice, event_tx: Sender<Event>) -> Result<Self> {
        let name = get_device_name(&device)?;
        let id = unsafe { device.GetId()?.to_string() }?;
        // windows-rs thinks the return value is DEVICE_STATE, but it's actually HRESULT
        // See https://github.com/microsoft/windows-rs/issues/3067
        let mut state: u32 = 0;
        #[allow(clippy::cast_possible_wrap)]
        let result = HRESULT(unsafe { device.GetState(&mut state) }.0 as _);
        let state = if result == S_OK {
            Self::translate_state(&device, DEVICE_STATE(state))?
        } else {
            return Err(windows::core::Error::new(
                result,
                "error getting device state",
            ));
        };
        let state_clone = state.clone();
        let mut device_info = Self {
            device,
            session_map: HashMap::new(),
            id,
            name,
            state,
            event_tx,
        };
        if let DeviceState::Active(_) = state_clone {
            device_info.activate()?;
        }
        Ok(device_info)
    }
    fn translate_state(device: &IMMDevice, new_state: DEVICE_STATE) -> Result<DeviceState> {
        Ok(match new_state {
            DEVICE_STATE_ACTIVE => {
                let audio_session_manager =
                    unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) }?;
                DeviceState::Active(audio_session_manager)
            }
            DEVICE_STATE_DISABLED => DeviceState::Disabled,
            DEVICE_STATE_NOTPRESENT => DeviceState::NotPresent,
            DEVICE_STATE_UNPLUGGED => DeviceState::Unplugged,
            _ => {
                return Err(windows::core::Error::new(
                    HRESULT(BAD_VALUE),
                    "Bad value for state",
                ))
            }
        })
    }
    fn set_state(&mut self, new_state: DEVICE_STATE) {
        self.state = Self::translate_state(&self.device, new_state).unwrap();
        if let DeviceState::Active(_) = self.state {
            self.activate().unwrap();
        }
    }
    fn activate(&mut self) -> Result<()> {
        let DeviceState::Active(audio_session_manager) = &self.state else {
            // Maybe the state changed again before we got here
            return Ok(());
        };
        // Register for notifications
        let audio_session_notification = AudioSessionNotification {
            device_id: self.id.clone(),
            event_tx: self.event_tx.clone(),
        };
        let audio_session_notification =
            IAudioSessionNotification::from(audio_session_notification);
        unsafe { audio_session_manager.RegisterSessionNotification(&audio_session_notification) }?;
        // The notifications won't start until we call `GetCount()` on the
        // session enumerator, so we do the below after the above
        let audio_session_collection = unsafe { audio_session_manager.GetSessionEnumerator() }?;
        for i in 0..unsafe { audio_session_collection.GetCount() }? {
            // Add the sessions to the map
            let session_control = unsafe { audio_session_collection.GetSession(i) }?;
            let session_control2 = session_control.cast::<IAudioSessionControl2>()?;
            let session_info = SessionInfo::new(
                self,
                unsafe { session_control2.GetSessionInstanceIdentifier()?.to_string() }?,
                session_control,
                session_control2,
                self.event_tx.clone(),
            )?;
            self.session_map
                .insert(session_info.instance_id.clone(), session_info);
        }
        Ok(())
    }
}

struct DeviceMap {
    map: HashMap<String, DeviceInfo>,
    defaults: [[Option<String>; 2]; 3],
}

impl DeviceMap {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            defaults: [[None, None], [None, None], [None, None]],
        }
    }
    fn get_default_device(&self, flow: EDataFlow, role: ERole) -> Option<&DeviceInfo> {
        assert_ne!(flow, EDataFlow::All);
        self.defaults[role as usize][flow as usize]
            .as_ref()
            .and_then(|id| self.map.get(id))
    }
}

#[implement(IAudioSessionNotification)]
struct AudioSessionNotification {
    device_id: String,
    event_tx: Sender<Event>,
}

impl IAudioSessionNotification_Impl for AudioSessionNotification {
    fn OnSessionCreated(&self, session: Option<&IAudioSessionControl>) -> Result<()> {
        let Some(session) = session else {
            return Ok(());
        };
        let session_control_2 = session.cast::<IAudioSessionControl2>()?;
        let session_instance_id = unsafe {
            session_control_2
                .GetSessionInstanceIdentifier()?
                .to_string()
        }?;
        self.event_tx
            .send(Event::Device(
                self.device_id.clone(),
                DeviceEvent::SessionCreated(session_instance_id),
            ))
            .unwrap();

        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct MMNotificationClient {
    event_tx: Sender<Event>,
}

impl IMMNotificationClient_Impl for MMNotificationClient {
    fn OnDefaultDeviceChanged(
        &self,
        flow: WindowsEDataFlow,
        role: WindowsERole,
        default_device_id: &PCWSTR,
    ) -> Result<()> {
        let flow = match flow {
            eRender => EDataFlow::Render,
            eCapture => EDataFlow::Capture,
            eAll => EDataFlow::All,
            _ => {
                return Err(windows::core::Error::new(
                    HRESULT(BAD_VALUE),
                    "Bad value for flow",
                ))
            }
        };
        let role = match role {
            eConsole => ERole::Console,
            eMultimedia => ERole::Multimedia,
            eCommunications => ERole::Communications,
            _ => {
                return Err(windows::core::Error::new(
                    HRESULT(BAD_VALUE),
                    "Bad value for role",
                ))
            }
        };
        let default_device_id_string = unsafe { default_device_id.to_string()? };
        self.event_tx
            .send(Event::Device(
                default_device_id_string,
                DeviceEvent::DefaultDeviceChanged(flow, role),
            ))
            .unwrap();
        Ok(())
    }

    fn OnDeviceAdded(&self, device_id: &PCWSTR) -> Result<()> {
        let device_id_string = unsafe { device_id.to_string()? };
        self.event_tx
            .send(Event::Device(device_id_string, DeviceEvent::DeviceAdded))
            .unwrap();
        Ok(())
    }

    fn OnDeviceRemoved(&self, device_id: &PCWSTR) -> Result<()> {
        let device_id_string = unsafe { device_id.to_string()? };
        self.event_tx
            .send(Event::Device(device_id_string, DeviceEvent::DeviceRemoved))
            .unwrap();
        Ok(())
    }

    fn OnDeviceStateChanged(&self, device_id: &PCWSTR, new_state: DEVICE_STATE) -> Result<()> {
        let device_id_string = unsafe { device_id.to_string()? };
        self.event_tx
            .send(Event::Device(
                device_id_string,
                DeviceEvent::DeviceStateChanged(new_state),
            ))
            .unwrap();
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _device_id: &PCWSTR, _key: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}

#[implement(IAudioSessionEvents)]
struct AudioSessionEvents {
    device_id: String,
    session_instance_id: String,
    event_tx: Sender<Event>,
}

impl IAudioSessionEvents_Impl for AudioSessionEvents {
    fn OnDisplayNameChanged(
        &self,
        new_display_name: &PCWSTR,
        _event_context: *const GUID,
    ) -> Result<()> {
        let new_display_name = unsafe { new_display_name.to_string()? };
        self.event_tx
            .send(Event::Session(
                self.device_id.clone(),
                self.session_instance_id.clone(),
                SessionEvent::DisplayNameChanged(new_display_name),
            ))
            .unwrap();
        Ok(())
    }

    fn OnIconPathChanged(&self, new_icon_path: &PCWSTR, _event_context: *const GUID) -> Result<()> {
        let new_icon_path = unsafe { new_icon_path.to_string()? };
        self.event_tx
            .send(Event::Session(
                self.device_id.clone(),
                self.session_instance_id.clone(),
                SessionEvent::IconPathChanged(new_icon_path),
            ))
            .unwrap();
        Ok(())
    }

    fn OnSimpleVolumeChanged(
        &self,
        new_volume: f32,
        new_mute: BOOL,
        _event_context: *const GUID,
    ) -> Result<()> {
        let new_mute = new_mute == TRUE;
        self.event_tx
            .send(Event::Session(
                self.device_id.clone(),
                self.session_instance_id.clone(),
                SessionEvent::SimpleVolumeChanged(SessionSimpleVolumeChangedEvent {
                    volume: new_volume,
                    mute: new_mute,
                }),
            ))
            .unwrap();
        Ok(())
    }

    // This is too low level for right now, we just care about the SimpleVolume
    // changes. As per
    // https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nn-audioclient-ichannelaudiovolume
    // channel volume is multiplied with simple volume and otherwise do not
    // influence eachother, so if we just ignore per channel volume, we're not
    // messing anything up either.
    fn OnChannelVolumeChanged(
        &self,
        _channel_count: u32,
        _new_channel_volume_array: *const f32,
        _changed_channel: u32,
        _event_context: *const GUID,
    ) -> Result<()> {
        Ok(())
    }

    fn OnGroupingParamChanged(
        &self,
        new_grouping_param: *const GUID,
        _event_context: *const GUID,
    ) -> Result<()> {
        let new_grouping_param = unsafe { new_grouping_param.as_ref() }.unwrap().to_u128();
        self.event_tx
            .send(Event::Session(
                self.device_id.clone(),
                self.session_instance_id.clone(),
                SessionEvent::GroupingParamChanged(new_grouping_param),
            ))
            .unwrap();
        Ok(())
    }

    fn OnStateChanged(&self, new_state: AudioSessionState) -> Result<()> {
        let new_state = SessionState::try_from(new_state)?;
        if new_state == SessionState::Expired {
            self.event_tx
                .send(Event::Session(
                    self.device_id.clone(),
                    self.session_instance_id.clone(),
                    SessionEvent::SessionDisconnected(DisconnectReason::SessionExpired),
                ))
                .unwrap();
        }
        self.event_tx
            .send(Event::Session(
                self.device_id.clone(),
                self.session_instance_id.clone(),
                SessionEvent::StateChanged(new_state),
            ))
            .unwrap();
        Ok(())
    }

    fn OnSessionDisconnected(&self, disconnect_reason: AudioSessionDisconnectReason) -> Result<()> {
        let disconnect_reason = DisconnectReason::try_from(disconnect_reason)?;
        self.event_tx
            .send(Event::Session(
                self.device_id.clone(),
                self.session_instance_id.clone(),
                SessionEvent::SessionDisconnected(disconnect_reason),
            ))
            .unwrap();
        Ok(())
    }
}

fn wide_string(input: &str) -> Vec<u16> {
    input.encode_utf16().chain(Some(0)).collect()
}

static EVENT_SENDER: OnceLock<Sender<Event>> = OnceLock::new();

fn main() -> Result<()> {
    let start = Instant::now();
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }?;
    let enumerator = unsafe {
        CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap()
    };
    println!("Time to initialize: {:?}", start.elapsed());

    let start = Instant::now();
    let (event_tx, event_rx) = mpsc::channel();
    let device_map = init_device_map(&enumerator, &event_tx)?;
    let device_map = Rc::new(Mutex::new(device_map));
    println!("Time to get devices: {:?}", start.elapsed());

    let start = Instant::now();
    print_devices(&device_map);
    println!("Time to print devices: {:?}", start.elapsed());

    let global_clone = event_tx.clone();
    // I see no other way to get this to the `win_event_hook_callback` callback
    EVENT_SENDER.get_or_init(move || global_clone);
    register_active_window_change()?;

    for event in event_rx {
        match event {
            Event::Device(device_id, device_event) => {
                handle_device_event(
                    &device_id,
                    device_event,
                    &device_map,
                    &enumerator,
                    &event_tx,
                )?;
            }
            Event::Session(device_id, session_instance_id, session_event) => {
                handle_session_event(&device_map, &device_id, &session_instance_id, session_event)?;
            }
            Event::ActiveWindowChange(pid) => {
                let system = System::new_with_specifics(
                    RefreshKind::new().with_processes(ProcessRefreshKind::new()),
                );
                let process = system.process(Pid::from_u32(pid));
                if let Some(proc) = process {
                    println!("Active Window: {}", proc.name());
                }
                let sessions = find_sessions_for_pid(pid, &device_map, &system);
                for (device_id, session_instance_id) in sessions {
                    let device_map_guard = device_map.lock().unwrap();
                    let device_info = device_map_guard.map.get(&device_id);
                    let Some(device_info) = device_info else {
                        drop(device_map_guard);
                        println!("Device not found: {device_id}");
                        continue;
                    };
                    let session_info = device_info.session_map.get(&session_instance_id);
                    let Some(session_info) = session_info else {
                        drop(device_map_guard);
                        println!("Session not found: {session_instance_id}");
                        continue;
                    };
                    println!(
                        "Active Window Session: Device={}, Session={}",
                        device_info.name,
                        session_info
                            .display_name
                            .as_ref()
                            .unwrap_or(&"Unknown".to_string())
                    );
                }
            }
        }
    }
    Ok(())
}

fn find_sessions_for_pid(
    pid: u32,
    device_map: &Rc<Mutex<DeviceMap>>,
    system: &System,
) -> Vec<(String, String)> {
    let device_map_guard = device_map.lock().unwrap();
    let proc_and_children = pid_and_child_pids(Pid::from_u32(pid), system);
    device_map_guard
        .map
        .values()
        .flat_map(|device| {
            device.session_map.values().filter_map(|session| {
                if proc_and_children.contains(&session.pid) {
                    Some((device.id.clone(), session.instance_id.clone()))
                } else {
                    None
                }
            })
        })
        .collect()
}

fn pid_and_child_pids(parent_pid: Pid, system: &System) -> HashSet<u32> {
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
    children.into_iter().flatten().map(Pid::as_u32).collect()
}

fn register_active_window_change() -> Result<HWINEVENTHOOK> {
    // The message loop that is required to receive the events needs to be in
    // the same thread as the one that calls SetWinEventHook. So make a thread
    // now that handles both.
    let (tx, rx) = oneshot::channel();
    //
    thread::spawn(move || {
        // Even though we don't (currently) produce audio from this app, we still
        // don't want to use WINEVENT_SKIPOWNPROCESS, because we want to know when
        // an audio producing app is no longer the active window.
        // WINEVENT_OUTCONTEXT because we aren't mapped into the address space of
        // any of the other processes.
        let flags = WINEVENT_OUTOFCONTEXT;
        let event_hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(win_event_hook_callback),
                0,
                0,
                flags,
            )
        };
        tx.send(event_hook).unwrap();
        // Grab current active window pid
        let foreground = unsafe { GetForegroundWindow() };
        let mut window_pid: u32 = 0;
        let _ = unsafe { GetWindowThreadProcessId(foreground, Some(&mut window_pid)) };
        EVENT_SENDER
            .get()
            .unwrap()
            .send(Event::ActiveWindowChange(window_pid))
            .unwrap();
        // This thread needs to own a window to receive messages
        let window = MessageLoopWindow::new().unwrap();
        let mut msg = MSG::default();
        loop {
            unsafe {
                let _ = GetMessageW(&mut msg, window.0, 0, 0);
            }
        }
    });
    let event_hook = rx.recv().unwrap();
    if event_hook.is_invalid() {
        return Err(Error::new(HRESULT::default(), "SetWinEventHook failed"));
    }
    Ok(event_hook)
}

unsafe extern "system" fn win_event_hook_callback(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if event != EVENT_SYSTEM_FOREGROUND {
        return;
    }
    let event_tx = EVENT_SENDER.get().unwrap();
    let mut pid: u32 = 0;
    let _ = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    event_tx.send(Event::ActiveWindowChange(pid)).unwrap();
}

struct MessageLoopWindow(HWND);

impl MessageLoopWindow {
    fn new() -> Result<Self> {
        let class_name = w!("STATIC");
        let window_name = w!("MessageLoopWindow");
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0), // no extended style
                class_name,
                window_name,
                WINDOW_STYLE(0), // no style
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                None,
                None,
            )
        };
        if window == HWND::default() {
            return Err(get_last_error());
        }
        Ok(Self(window))
    }
}

fn get_last_error() -> Error {
    let error = unsafe { GetLastError() };
    let flags = FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS;
    let mut buf = [0u16; 256];
    let message: PWSTR = PWSTR(buf.as_mut_ptr());
    unsafe {
        FormatMessageW(flags, None, error.0, LANG_NEUTRAL, message, 256, None);
    }
    //let message: *mut PWSTR = unsafe { transmute(message) };
    let message = unsafe { message.to_string() }.unwrap();
    Error::new(HRESULT::from(error), message)
}

impl Drop for MessageLoopWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0).unwrap();
        }
    }
}

fn handle_session_event(
    device_map: &Mutex<DeviceMap>,
    device_id: &str,
    session_instance_id: &str,
    session_event: SessionEvent,
) -> Result<()> {
    let mut device_map_guard = device_map.lock().unwrap();
    let device_info = device_map_guard.map.get_mut(device_id);
    let Some(device_info) = device_info else {
        drop(device_map_guard);
        println!("Device not found: {device_id}");
        return Ok(());
    };
    let session_info = device_info.session_map.get_mut(session_instance_id);
    let Some(session_info) = session_info else {
        println!(
            "Sessions in {}: {:?}",
            device_info.name,
            device_info.session_map.keys()
        );
        drop(device_map_guard);
        println!("Session not found: {session_instance_id}");
        return Ok(());
    };
    let device_name = device_info.name.clone();
    let session_name = session_info
        .display_name
        .as_deref()
        .unwrap_or("Unknown")
        .to_string();
    match session_event {
        SessionEvent::SimpleVolumeChanged(event) => {
            drop(device_map_guard);
            println!(
                "Simple Volume Changed: Device={}, Session={}, Volume={}, Mute={}",
                device_name, session_name, event.volume, event.mute
            );
        }
        SessionEvent::DisplayNameChanged(new_display_name) => {
            session_info.set_display_name(Some(new_display_name.clone()))?;
            drop(device_map_guard);
            println!(
                "New Display Name: Device={device_name}, Session={session_name}, Name={new_display_name}"
            );
        }
        SessionEvent::GroupingParamChanged(new_grouping_param) => {
            drop(device_map_guard);
            println!(
                "New Grouping Param: Device={device_name}, Session={session_name}, Param={new_grouping_param}"
            );
        }
        SessionEvent::IconPathChanged(new_icon_path) => {
            drop(device_map_guard);
            println!(
                "New Icon Path: Device={device_name}, Session={session_name}, Path={new_icon_path}"
            );
        }
        SessionEvent::StateChanged(new_state) => {
            drop(device_map_guard);
            println!("New State: Device={device_name}, Session={session_name}, State={new_state}");
        }
        SessionEvent::SessionDisconnected(disconnect_reason) => {
            device_info.session_map.remove(session_instance_id);
            drop(device_map_guard);
            println!(
                "Session Disconnected: Device={device_name}, Session={session_name}, Reason={disconnect_reason}"
            );
        }
    }
    Ok(())
}

fn print_devices(device_map: &Mutex<DeviceMap>) {
    println!("Devices:");
    let device_map_guard = device_map.lock().unwrap();
    device_map_guard
        .map
        .iter()
        .filter(|(_, info)| matches!(info.state, DeviceState::Active(_)))
        .for_each(|(id, info)| {
            println!("  {}: {:?}", id, info.name);
            //            info.session_map.iter().for_each(|(id, session)| {
            //                println!("    {}: {:?} [{}]", id, session.display_name, session.state);
            //            });
        });
    println!("Default Devices:");
    for role in [ERole::Console, ERole::Multimedia, ERole::Communications] {
        for flow in [EDataFlow::Render, EDataFlow::Capture] {
            let device_info = device_map_guard.get_default_device(flow, role);
            println!("  {}: {:?}", role, device_info.map(|info| &info.name));
        }
    }
}

fn init_device_map(
    enumerator: &IMMDeviceEnumerator,
    event_tx: &Sender<Event>,
) -> Result<DeviceMap> {
    let start = Instant::now();
    let mut device_map = DeviceMap::new();
    let notification_client = IMMNotificationClient::from(MMNotificationClient {
        event_tx: event_tx.clone(),
    });
    unsafe { enumerator.RegisterEndpointNotificationCallback(&notification_client) }?;
    println!("Time to register callback: {:?}", start.elapsed());
    let start = Instant::now();
    device_map
        .map
        .extend(all_devices(enumerator, event_tx.clone())?.filter_map(Result::ok));
    println!("Time to get all devices: {:?}", start.elapsed());
    let start = Instant::now();
    for flow in [EDataFlow::Render, EDataFlow::Capture] {
        for role in [ERole::Console, ERole::Multimedia, ERole::Communications] {
            let win_flow = flow.into();
            let win_role = role.into();
            let device_id = unsafe {
                enumerator
                    .GetDefaultAudioEndpoint(win_flow, win_role)?
                    .GetId()?
                    .to_string()
            }?;
            device_map.defaults[role as usize][flow as usize] = Some(device_id);
        }
    }
    println!("Time to get default devices: {:?}", start.elapsed());
    Ok(device_map)
}

fn handle_device_event(
    device_id: &str,
    device_event: DeviceEvent,
    device_map: &Mutex<DeviceMap>,
    enumerator: &IMMDeviceEnumerator,
    event_tx: &Sender<Event>,
) -> Result<()> {
    match device_event {
        DeviceEvent::DefaultDeviceChanged(flow, role) => {
            let mut device_map_guard = device_map.lock().unwrap();
            let device_info = device_map_guard.map.get(device_id);
            let Some(device_info) = device_info else {
                drop(device_map_guard);
                println!("Device not found: {device_id}");
                return Ok(());
            };
            let name = device_info.name.clone();
            let flows = match flow {
                EDataFlow::All => [Some(EDataFlow::Render), Some(EDataFlow::Capture)],
                _ => [Some(flow), None],
            };
            for flow in flows.into_iter().flatten() {
                device_map_guard.defaults[role as usize][flow as usize] =
                    Some(device_id.to_string());
            }
            drop(device_map_guard);
            println!("Default device changed: Flow={flow:?}, Role={role:?}, Device={name}",);
        }
        DeviceEvent::DeviceAdded => {
            let device_id_vec = wide_string(device_id);
            let device_id = PCWSTR(device_id_vec.as_ptr());
            let device = unsafe { enumerator.GetDevice(device_id) }?;
            let device_info = DeviceInfo::new(device, event_tx.clone())?;
            println!("Device added: {:?}", device_info.name);
            let mut device_map_guard = device_map.lock().unwrap();
            device_map_guard
                .map
                .insert(device_info.id.clone(), device_info);
        }
        DeviceEvent::DeviceRemoved => {
            let mut device_map_guard = device_map.lock().unwrap();
            let removed = device_map_guard.map.remove(device_id);
            drop(device_map_guard);
            if let Some(removed) = removed {
                println!("Device removed: {:?}", removed.name);
            } else {
                println!("Device not found in map");
            }
        }
        DeviceEvent::DeviceStateChanged(new_state) => {
            let mut device_map_guard = device_map.lock().unwrap();
            let device_info = device_map_guard.map.get_mut(device_id);
            let Some(device_info) = device_info else {
                drop(device_map_guard);
                println!("Device not found: {device_id}");
                return Ok(());
            };
            let old_state = format!("{}", device_info.state);
            device_info.set_state(new_state);
            let name = device_info.name.clone();
            let new_state = format!("{}", device_info.state);
            drop(device_map_guard);
            println!(
                "Device state changed: Device={name}, OldState={old_state}, NewState={new_state}"
            );
        }
        DeviceEvent::SessionCreated(session_instance_id) => {
            let mut device_map_guard = device_map.lock().unwrap();
            let Some(device_info) = device_map_guard.map.get_mut(device_id) else {
                drop(device_map_guard);
                println!("Device not found: {device_id}");
                return Ok(());
            };
            let device_name = device_info.name.clone();
            let DeviceState::Active(session_manager_2) = &device_info.state else {
                drop(device_map_guard);
                println!("Device not active: {device_name}");
                return Ok(());
            };
            let session = all_sessions(session_manager_2)?.find_map(|item| {
                let Ok((siid, (c, c2))) = item else {
                    return None;
                };
                if siid == session_instance_id {
                    Some(SessionInfo::new(device_info, siid, c, c2, event_tx.clone()).unwrap())
                } else {
                    None
                }
            });
            let Some(session) = session else {
                drop(device_map_guard);
                println!("Session not found: {session_instance_id}");
                return Ok(());
            };
            device_info
                .session_map
                .insert(session_instance_id.clone(), session);
            drop(device_map_guard);
            println!("Session created on {device_name}: {session_instance_id}");
        }
    }
    Ok(())
}

fn all_devices(
    enumerator: &IMMDeviceEnumerator,
    event_tx: Sender<Event>,
) -> Result<impl Iterator<Item = Result<(String, DeviceInfo)>>> {
    let all_states = DEVICE_STATE(
        DEVICE_STATE_ACTIVE.0
            | DEVICE_STATE_DISABLED.0
            | DEVICE_STATE_NOTPRESENT.0
            | DEVICE_STATE_UNPLUGGED.0,
    );
    let devices = unsafe { enumerator.EnumAudioEndpoints(eAll, all_states) }?;
    Ok((0..unsafe { devices.GetCount() }?).map(move |i| {
        let device = unsafe { devices.Item(i) }?;
        let id = unsafe { device.GetId()?.to_string() }?;
        let device_info = DeviceInfo::new(device, event_tx.clone())?;
        Ok((id, device_info))
    }))
}

fn all_sessions(
    session_manager_2: &IAudioSessionManager2,
) -> Result<impl Iterator<Item = Result<(String, (IAudioSessionControl, IAudioSessionControl2))>>> {
    let session_collection = unsafe { session_manager_2.GetSessionEnumerator() }?;
    Ok(
        (0..unsafe { session_collection.GetCount() }?).map(move |i| {
            let session_control = unsafe { session_collection.GetSession(i) }?;
            let session_control_2 = session_control.cast::<IAudioSessionControl2>()?;
            let session_instance_id = unsafe {
                session_control_2
                    .GetSessionInstanceIdentifier()?
                    .to_string()
            }?;
            Ok((session_instance_id, (session_control, session_control_2)))
        }),
    )
}

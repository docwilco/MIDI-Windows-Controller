use std::{ptr, thread::sleep, time::Duration};
use sysinfo::{Pid, System};
use windows::{
    core::Error,
    Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};

fn main() -> Result<(), Error> {
    let mut system = System::new_all();
    loop {
        unsafe {
            let foreground = GetForegroundWindow();
            let mut pid: u32 = 0;
            let _ = GetWindowThreadProcessId(foreground, Some(ptr::addr_of_mut!(pid)));
            system.refresh_processes();
            let mut process = system.process(Pid::from_u32(pid));
            let mut indent = 0;
            while let Some(proc) = process {
                println!(
                    "{:indent$}{} {:?}",
                    "",
                    proc.pid(),
                    proc.name(),
                    indent = indent
                );
                process = proc.parent().and_then(|pid| system.process(pid));
                indent += 1;
            }
            sleep(Duration::from_secs(1));
        }
    }
}

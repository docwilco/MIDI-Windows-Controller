use derive_more::From;
use midir::MidiInput;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    DeviceNotFound,
    // -- Externals
    #[from]
    Dotenv(dotenvy::Error),
    #[from]
    MspcReceive(std::sync::mpsc::RecvError),
    #[from]
    MspcSend(std::sync::mpsc::SendError<()>),
    #[from]
    MidiConnect(midir::ConnectError<MidiInput>),
    #[from]
    MidiInit(midir::InitError),
    #[from]
    Windows(windows::core::Error),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate

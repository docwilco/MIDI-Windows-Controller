use enum_dispatch::enum_dispatch;
use midly::{
    live::{LiveEvent, MtcQuarterFrameMessage, SystemCommon, SystemRealtime},
    num::{u14, u4, u7},
    MidiMessage, PitchBend,
};

use crate::MidiBytes;

use super::Control;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ValueMatchType {
    Exact,
    ThresholdOrAbove,
    ThresholdOrBelow,
}

#[enum_dispatch]
pub(crate) trait Trigger {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool;
}

#[derive(Debug)]
pub(crate) struct TriggerNoteOn {
    pub(crate) channel: u4,
    pub(crate) note: u7,
    pub(crate) velocity: u7,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerNoteOn {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Midi {
            channel,
            message: MidiMessage::NoteOn { key, vel },
        } = event
        {
            if self.channel == *channel && self.note == *key {
                return match self.match_type {
                    ValueMatchType::Exact => *vel == self.velocity,
                    ValueMatchType::ThresholdOrAbove => *vel >= self.velocity,
                    ValueMatchType::ThresholdOrBelow => *vel <= self.velocity,
                };
            }
        }
        false
    }
}

impl Control for TriggerNoteOn {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerNoteOn: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Midi {
                    channel: self.channel,
                    message: MidiMessage::NoteOn {
                        key: self.note,
                        vel: u7::default(),
                    },
                })
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Midi {
                channel: self.channel,
                message: MidiMessage::NoteOn {
                    key: self.note,
                    vel: self.velocity,
                },
            });
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerNoteOff {
    pub(crate) channel: u4,
    pub(crate) note: u7,
    pub(crate) velocity: u7,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerNoteOff {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Midi {
            channel,
            message: MidiMessage::NoteOff { key, vel },
        } = event
        {
            if self.channel == *channel && self.note == *key {
                return match self.match_type {
                    ValueMatchType::Exact => *vel == self.velocity,
                    ValueMatchType::ThresholdOrAbove => *vel >= self.velocity,
                    ValueMatchType::ThresholdOrBelow => *vel <= self.velocity,
                };
            }
        }
        false
    }
}

impl Control for TriggerNoteOff {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerNoteOff: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Midi {
                    channel: self.channel,
                    message: MidiMessage::NoteOff {
                        key: self.note,
                        vel: u7::default(),
                    },
                })
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Midi {
                channel: self.channel,
                message: MidiMessage::NoteOff {
                    key: self.note,
                    vel: self.velocity,
                },
            });
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerAftertouch {
    pub(crate) channel: u4,
    pub(crate) note: u7,
    pub(crate) pressure: u7,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerAftertouch {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Midi {
            channel,
            message: MidiMessage::Aftertouch { key, vel },
        } = event
        {
            if self.channel == *channel && self.note == *key {
                return match self.match_type {
                    ValueMatchType::Exact => *vel == self.pressure,
                    ValueMatchType::ThresholdOrAbove => *vel >= self.pressure,
                    ValueMatchType::ThresholdOrBelow => *vel <= self.pressure,
                };
            }
        }
        false
    }
}

impl Control for TriggerAftertouch {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerAftertouch: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Midi {
                    channel: self.channel,
                    message: MidiMessage::Aftertouch {
                        key: self.note,
                        vel: u7::default(),
                    },
                })
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Midi {
                channel: self.channel,
                message: MidiMessage::Aftertouch {
                    key: self.note,
                    vel: self.pressure,
                },
            });
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerController {
    pub(crate) channel: u4,
    pub(crate) controller: u7,
    pub(crate) value: u7,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerController {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Midi {
            channel,
            message: MidiMessage::Controller { controller, value },
        } = event
        {
            if self.channel == *channel && self.controller == *controller {
                return match self.match_type {
                    ValueMatchType::Exact => *value == self.value,
                    ValueMatchType::ThresholdOrAbove => *value >= self.value,
                    ValueMatchType::ThresholdOrBelow => *value <= self.value,
                };
            }
        }
        false
    }
}

impl Control for TriggerController {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerController: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Midi {
                    channel: self.channel,
                    message: MidiMessage::Controller {
                        controller: self.controller,
                        value: u7::default(),
                    },
                })
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Midi {
                channel: self.channel,
                message: MidiMessage::Controller {
                    controller: self.controller,
                    value: self.value,
                },
            });
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerProgramChange {
    pub(crate) channel: u4,
    pub(crate) program: u7,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerProgramChange {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Midi {
            channel,
            message: MidiMessage::ProgramChange { program },
        } = event
        {
            if self.channel == *channel {
                return match self.match_type {
                    ValueMatchType::Exact => *program == self.program,
                    ValueMatchType::ThresholdOrAbove => *program >= self.program,
                    ValueMatchType::ThresholdOrBelow => *program <= self.program,
                };
            }
        }
        false
    }
}

impl Control for TriggerProgramChange {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerProgramChange: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Midi {
                    channel: self.channel,
                    message: MidiMessage::ProgramChange {
                        program: u7::default(),
                    },
                })
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Midi {
                channel: self.channel,
                message: MidiMessage::ProgramChange {
                    program: self.program,
                },
            });
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerChannelAftertouch {
    pub(crate) channel: u4,
    pub(crate) pressure: u7,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerChannelAftertouch {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Midi {
            channel,
            message: MidiMessage::ChannelAftertouch { vel },
        } = event
        {
            if self.channel == *channel {
                return match self.match_type {
                    ValueMatchType::Exact => *vel == self.pressure,
                    ValueMatchType::ThresholdOrAbove => *vel >= self.pressure,
                    ValueMatchType::ThresholdOrBelow => *vel <= self.pressure,
                };
            }
        }
        false
    }
}

impl Control for TriggerChannelAftertouch {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerChannelAftertouch: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Midi {
                    channel: self.channel,
                    message: MidiMessage::ChannelAftertouch { vel: u7::default() },
                })
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Midi {
                channel: self.channel,
                message: MidiMessage::ChannelAftertouch { vel: self.pressure },
            });
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerPitchBend {
    pub(crate) channel: u4,
    pub(crate) value: i16,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerPitchBend {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Midi {
            channel,
            message: MidiMessage::PitchBend { bend },
        } = event
        {
            if self.channel == *channel {
                return match self.match_type {
                    ValueMatchType::Exact => bend.as_int() == self.value,
                    ValueMatchType::ThresholdOrAbove => bend.as_int() >= self.value,
                    ValueMatchType::ThresholdOrBelow => bend.as_int() <= self.value,
                };
            }
        }
        false
    }
}

impl Control for TriggerPitchBend {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerPitchBend: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Midi {
                    channel: self.channel,
                    message: MidiMessage::PitchBend {
                        bend: PitchBend::mid_raw_value(),
                    },
                })
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Midi {
                channel: self.channel,
                message: MidiMessage::PitchBend {
                    bend: PitchBend::from_int(self.value),
                },
            });
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerMtcQuarterFrame {
    pub(crate) message: MtcQuarterFrameMessage,
    pub(crate) value: u4,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerMtcQuarterFrame {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Common(SystemCommon::MidiTimeCodeQuarterFrame(message, value)) = event {
            if *message == self.message {
                return match self.match_type {
                    ValueMatchType::Exact => *value == self.value,
                    ValueMatchType::ThresholdOrAbove => *value >= self.value,
                    ValueMatchType::ThresholdOrBelow => *value <= self.value,
                };
            }
        }
        false
    }
}

impl Control for TriggerMtcQuarterFrame {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerMtcQuarterFrame: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Common(SystemCommon::MidiTimeCodeQuarterFrame(
                    self.message,
                    u4::default(),
                )))
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Common(SystemCommon::MidiTimeCodeQuarterFrame(
                self.message,
                self.value,
            )));
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerSongPosition {
    pub(crate) position: u14,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerSongPosition {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Common(SystemCommon::SongPosition(position)) = event {
            return match self.match_type {
                ValueMatchType::Exact => *position == self.position,
                ValueMatchType::ThresholdOrAbove => *position >= self.position,
                ValueMatchType::ThresholdOrBelow => *position <= self.position,
            };
        }
        false
    }
}

impl Control for TriggerSongPosition {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerSongPosition: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => Some(
                LiveEvent::Common(SystemCommon::SongPosition(u14::default())),
            ),
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Common(SystemCommon::SongPosition(self.position)));
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerSongSelect {
    pub(crate) song: u7,
    pub(crate) match_type: ValueMatchType,
}

impl Trigger for TriggerSongSelect {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        if let LiveEvent::Common(SystemCommon::SongSelect(song)) = event {
            return match self.match_type {
                ValueMatchType::Exact => *song == self.song,
                ValueMatchType::ThresholdOrAbove => *song >= self.song,
                ValueMatchType::ThresholdOrBelow => *song <= self.song,
            };
        }
        false
    }
}

impl Control for TriggerSongSelect {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerSongSelect: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        match self.match_type {
            ValueMatchType::ThresholdOrAbove | ValueMatchType::ThresholdOrBelow => {
                Some(LiveEvent::Common(SystemCommon::SongSelect(u7::default())))
            }
            ValueMatchType::Exact => None,
        }
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        if self.match_type == ValueMatchType::Exact {
            return Some(LiveEvent::Common(SystemCommon::SongSelect(self.song)));
        }
        None
    }
}

#[derive(Debug)]
pub(crate) struct TriggerTuneRequest {}

impl Trigger for TriggerTuneRequest {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        matches!(event, LiveEvent::Common(SystemCommon::TuneRequest))
    }
}

impl Control for TriggerTuneRequest {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerTuneRequest: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        None
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        Some(LiveEvent::Common(SystemCommon::TuneRequest))
    }
}

#[derive(Debug)]
pub(crate) struct TriggerTimingClock {}

impl Trigger for TriggerTimingClock {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        matches!(event, LiveEvent::Realtime(SystemRealtime::TimingClock))
    }
}

impl Control for TriggerTimingClock {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerTimingClock: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        None
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        Some(LiveEvent::Realtime(SystemRealtime::TimingClock))
    }
}

#[derive(Debug)]
pub(crate) struct TriggerStart {}

impl Trigger for TriggerStart {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        matches!(event, LiveEvent::Realtime(SystemRealtime::Start))
    }
}

impl Control for TriggerStart {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerStart: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        None
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        Some(LiveEvent::Realtime(SystemRealtime::Start))
    }
}

#[derive(Debug)]
pub(crate) struct TriggerContinue {}

impl Trigger for TriggerContinue {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        matches!(event, LiveEvent::Realtime(SystemRealtime::Continue))
    }
}

impl Control for TriggerContinue {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerContinue: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        None
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        Some(LiveEvent::Realtime(SystemRealtime::Continue))
    }
}

#[derive(Debug)]
pub(crate) struct TriggerStop {}

impl Trigger for TriggerStop {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        matches!(event, LiveEvent::Realtime(SystemRealtime::Stop))
    }
}

impl Control for TriggerStop {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerStop: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        None
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        Some(LiveEvent::Realtime(SystemRealtime::Stop))
    }
}

#[derive(Debug)]
pub(crate) struct TriggerActiveSensing {}

impl Trigger for TriggerActiveSensing {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        matches!(event, LiveEvent::Realtime(SystemRealtime::ActiveSensing))
    }
}

impl Control for TriggerActiveSensing {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerActiveSensing: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        None
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        Some(LiveEvent::Realtime(SystemRealtime::ActiveSensing))
    }
}

#[derive(Debug)]
pub(crate) struct TriggerReset {}

impl Trigger for TriggerReset {
    fn is_triggered_by(&self, event: &LiveEvent) -> bool {
        matches!(event, LiveEvent::Realtime(SystemRealtime::Reset))
    }
}

impl Control for TriggerReset {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        if self.is_triggered_by(event) {
            println!("TriggerReset: {event:?}");
        }
    }

    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        None
    }

    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        Some(LiveEvent::Realtime(SystemRealtime::Reset))
    }
}

#[enum_dispatch(Control)]
#[derive(Debug)]
pub(crate) enum TriggerMidiMessage {
    // MIDI
    NoteOn(TriggerNoteOn),
    NoteOff(TriggerNoteOff),
    Aftertouch(TriggerAftertouch),
    Controller(TriggerController),
    ProgramChange(TriggerProgramChange),
    ChannelAftertouch(TriggerChannelAftertouch),
    PitchBend(TriggerPitchBend),
    // System Common
    MtcQuarterFrame(TriggerMtcQuarterFrame),
    SongPosition(TriggerSongPosition),
    SongSelect(TriggerSongSelect),
    TuneRequest(TriggerTuneRequest),
    // System Real-Time
    TimingClock(TriggerTimingClock),
    Start(TriggerStart),
    Continue(TriggerContinue),
    Stop(TriggerStop),
    ActiveSensing(TriggerActiveSensing),
    Reset(TriggerReset),
}

pub(crate) fn live_event_without_value(event: &[u8]) -> MidiBytes {
    let mut event = LiveEvent::parse(event).unwrap();
    match event {
        LiveEvent::Midi {
            channel: _,
            ref mut message,
        } => match message {
            MidiMessage::NoteOn {
                key: _,
                vel: ref mut value,
            }
            | MidiMessage::NoteOff {
                key: _,
                vel: ref mut value,
            }
            | MidiMessage::Aftertouch {
                key: _,
                vel: ref mut value,
            }
            | MidiMessage::Controller {
                controller: _,
                ref mut value,
            } => *value = u7::default(),
            MidiMessage::ProgramChange { ref mut program } => *program = u7::default(),
            MidiMessage::ChannelAftertouch { ref mut vel } => *vel = u7::default(),
            MidiMessage::PitchBend { ref mut bend } => *bend = PitchBend::mid_raw_value(),
        },
        LiveEvent::Common(ref mut system_common) => match system_common {
            SystemCommon::MidiTimeCodeQuarterFrame(_, ref mut value) => *value = u4::default(),
            SystemCommon::SongPosition(ref mut value) => *value = u14::default(),
            SystemCommon::SongSelect(ref mut value) => *value = u7::default(),
            _ => (),
        },
        LiveEvent::Realtime(_) => (),
    }
    event.into()
}

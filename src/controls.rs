use crate::MidiBytes;
use enum_dispatch::enum_dispatch;
use midly::live::LiveEvent;
use trigger::TriggerMidiMessage;

pub(crate) mod trigger;
use trigger::{
    TriggerActiveSensing, TriggerAftertouch, TriggerChannelAftertouch, TriggerContinue,
    TriggerController, TriggerMtcQuarterFrame, TriggerNoteOff, TriggerNoteOn, TriggerPitchBend,
    TriggerProgramChange, TriggerReset, TriggerSongPosition, TriggerSongSelect, TriggerStart,
    TriggerStop, TriggerTimingClock, TriggerTuneRequest,
};
pub(crate) mod indicator;

//enum Direction {
//    Up,
//    Down,
//}

#[enum_dispatch]
pub(crate) trait Control {
    fn handle_midi_event_inner(&self, event: &LiveEvent);
    fn threshold_hash_key_inner(&self) -> Option<LiveEvent>;
    fn exact_hash_key_inner(&self) -> Option<LiveEvent>;

    fn handle_midi_event(&self, message: &[u8]) {
        let event = LiveEvent::parse(message).unwrap();
        self.handle_midi_event_inner(&event);
    }
    fn threshold_hash_key(&self) -> Option<MidiBytes> {
        self.threshold_hash_key_inner().map(Into::into)
    }
    fn exact_hash_key(&self) -> Option<MidiBytes> {
        self.exact_hash_key_inner().map(Into::into)
    }
}

#[derive(Debug)]
pub(crate) struct TriggerConfig {
    pub(crate) command: TriggerMidiMessage,
    pub(crate) _auto_indicate: bool,
}

impl Control for TriggerConfig {
    fn handle_midi_event_inner(&self, event: &LiveEvent) {
        self.command.handle_midi_event_inner(event);
    }
    fn exact_hash_key_inner(&self) -> Option<LiveEvent> {
        self.command.exact_hash_key_inner()
    }
    fn threshold_hash_key_inner(&self) -> Option<LiveEvent> {
        self.command.threshold_hash_key_inner()
    }
}

//struct RelativeValue {
//    command: MidiMessageMatch,
//    steps: u16,
//    up_value: u8,
//    up_direction: Direction,
//    down_value: u8,
//    down_direction: Direction,
//}
//
//struct AbsoluteValue {
//    command: MidiMessageMatch,
//    control: u7,
//    min: u14,
//    max: u14,
//}
//
//struct Indicator {
//    command: MidiMessageMatch,
//    min: u14,
//    max: u14,
//}

#[derive(Debug)]
#[enum_dispatch(Control)]
pub(crate) enum ControlType {
    Trigger(TriggerConfig),
    //    AbsoluteValue(AbsoluteValue),
    //    RelativeValue(RelativeValue),
    //    Indicator(Indicator),
}

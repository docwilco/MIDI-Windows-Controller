mod controls;
mod error;
use std::{collections::HashMap, ops::Deref, sync::Arc};

use controls::{
    trigger::{
        live_event_without_value, TriggerMidiMessage, TriggerNoteOn, ValueMatchType,
    },
    Control,
    TriggerConfig,
};
use error::{Error, Result};
use log::debug;
use midir::MidiInput;
use midly::{io::IoWrap, live::LiveEvent, num::{u4, u7}};
use smallvec::SmallVec;
mod midi;
mod windows_audio;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct MidiBytes(SmallVec<[u8; 3]>); // 3 bytes is the common size for midi messages

impl MidiBytes {
    fn from_slice(slice: &[u8]) -> Self {
        Self(SmallVec::from_slice(slice))
    }
}

impl From<LiveEvent<'_>> for MidiBytes {
    fn from(event: LiveEvent) -> Self {
        let bytes = SmallVec::new();
        let mut wrap = IoWrap(bytes);
        event.write(&mut wrap).unwrap();
        MidiBytes(wrap.0)
    }
}

impl Deref for MidiBytes {
    type Target = SmallVec<[u8; 3]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn main() -> Result<()> {
    dotenvy::dotenv()?;
    env_logger::init();
    let (midi_input_tx, midi_input_rx) = std::sync::mpsc::channel();
    let mut exact_midi_events = HashMap::new();
    let mut threshold_midi_events = HashMap::new();
    let button1 = Arc::new(controls::ControlType::Trigger(TriggerConfig {
        command: TriggerMidiMessage::NoteOn(TriggerNoteOn {
            channel: u4::from(0),
            note: u7::from(0x59),
            velocity: u7::from(0x7F),
            match_type: ValueMatchType::Exact,
        }),
        _auto_indicate: false,
    }));
    let button2 = Arc::new(controls::ControlType::Trigger(TriggerConfig {
        command: TriggerMidiMessage::NoteOn(TriggerNoteOn {
            channel: u4::from(0),
            note: u7::from(0x5A),
            velocity: u7::from(0x0F),
            match_type: ValueMatchType::ThresholdOrAbove,
        }),
        _auto_indicate: false,
    }));
    let button3 = Arc::new(controls::ControlType::Trigger(TriggerConfig {
        command: TriggerMidiMessage::NoteOn(TriggerNoteOn {
            channel: u4::from(0),
            note: u7::from(0x5B),
            velocity: u7::from(0x7F),
            match_type: ValueMatchType::ThresholdOrBelow,
        }),
        _auto_indicate: false,
    }));
    let controls = vec![button1, button2, button3];
    for control in controls {
        if let Some(exact_key) = control.exact_hash_key() {
            exact_midi_events.insert(exact_key, vec![control.clone()]);
        }
        if let Some(threshold_key) = control.threshold_hash_key() {
            threshold_midi_events.insert(threshold_key, vec![control.clone()]);
        }
    }
    let midi_in = MidiInput::new("MIDI Windows Controller")?;
    let in_ports = midi_in.ports();
    let in_port = in_ports.iter().find(|port| {
        midi_in
            .port_name(port)
            .map_or(false, |name| name == "X-TOUCH MINI")
    });
    let in_port = in_port.ok_or(Error::DeviceNotFound)?;
    let _conn = midi_in.connect(
        in_port,
        "event-listener",
        |_ts, message, midi_input_tx| {
            debug!("Received midi message: {:?}", message);
            let message = MidiBytes::from_slice(message);
            midi_input_tx
                .send(message)
                .expect("Failed to send midi event to processing thread");
        },
        midi_input_tx,
    )?;
    debug!("Maps: {:?}", exact_midi_events);
    loop {
        let bytes: MidiBytes = midi_input_rx.recv()?.into();
        debug!("Received midi event: {:?}", bytes);
        let triggers = exact_midi_events.get(&bytes);
        for trigger in triggers.into_iter().flatten() {
            trigger.handle_midi_event(&bytes);
        }
        let event_without_value = live_event_without_value(&bytes);
        let triggers = threshold_midi_events.get(&event_without_value);
        for trigger in triggers.into_iter().flatten() {
            trigger.handle_midi_event(&bytes);
        }
    }
}

use std::{thread::sleep, time::Duration};

use midir::{MidiInput, MidiOutput};
use midly::{live::LiveEvent, MidiMessage};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    // List midi devices
    let midi_out = MidiOutput::new("MIDI Windows Controller")?;
    let outports = midi_out.ports();
    for outport in &outports {
        println!("Output port: {}", midi_out.port_name(outport)?);
    }
    let outport = outports.iter().find(|port| {
        midi_out
            .port_name(port)
            .map_or(false, |name| name == "X-TOUCH MINI")
    });
    println!("outport");
    let Some(outport) = outport else {
        return Err("No output port found".into());
    };
    let _conn = midi_out.connect(outport, "midir-test")?;

    let midi_in = MidiInput::new("MIDI Windows Controller")?;
    let inports = midi_in.ports();
    for inport in &inports {
        println!("Input port: {}", midi_in.port_name(inport)?);
    }
    let inport = inports.iter().find(|port| {
        midi_in
            .port_name(port)
            .map_or(false, |name| name == "X-TOUCH MINI")
    });
    let Some(inport) = inport else {
        return Err("No input port found".into());
    };
    let _conn = midi_in.connect(
        inport,
        "midir-test",
        |_, message, ()| {
            let event = LiveEvent::parse(message).unwrap();
            let LiveEvent::Midi {
                channel,
                mut message,
            } = event
            else {
                return;
            };
            if let MidiMessage::NoteOn { key, vel } = message {
                if vel == 0 {
                    message = MidiMessage::NoteOff { key, vel };
                }
            }
            println!("ch{channel}: {message:?}");
        },
        (),
    )?;
    sleep(Duration::from_secs(10000));
    Ok(())
}

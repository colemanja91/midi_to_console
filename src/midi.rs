use std::convert::TryFrom;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use log::{error, info, trace};

use midir::{Ignore, MidiInput};

/// This thread has infinite loop in the end to process midi forever
pub fn process_signals(position: usize, tx: Sender<Vec<MidiMessageData>>) -> Result<(), Box<dyn Error>> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    let in_ports = midi_in.ports();

    let in_port = match in_ports.len() {
        0 => return Err("no input port found".into()),
        1 => {
            info!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            info!("Available input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                info!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            in_ports
                .get(position)
                .ok_or("invalid MIDI input port selected")?
        }
    };

    let in_port_name = midi_in.port_name(in_port)?;
    info!("Connecting to {}", in_port_name);

    // Make the persistent note-on message list shared and thread-safe so it can be
    // inspected or modified from multiple places in future refactors.
    let midi_note_on_messages: Arc<Mutex<Vec<MidiMessageData>>> = Arc::new(Mutex::new(Vec::new()));
    let state_for_callback = midi_note_on_messages.clone();

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in.connect(
        in_port,
        "midir-read-input",
        move |_, message: &[u8], _| {
            // Grab a local copy of the persistent state, run the pure processing
            // logic (process_callback) and then store the updated state back into
            // the mutex. This keeps the mutex held only for the clone/replace
            // and leaves the processing and sends inside process_callback to run
            // without holding the lock for the whole time.
            let persistent = {
                match state_for_callback.lock() {
                    Ok(guard) => guard.clone(),
                    Err(poisoned) => poisoned.into_inner().clone(),
                }
            };

            match process_callback(message, persistent, tx.clone()) {
                Ok(value) => {
                    // Replace the contents of the mutex with the updated state
                    match state_for_callback.lock() {
                        Ok(mut guard) => *guard = value,
                        Err(poisoned) => {
                            let mut guard = poisoned.into_inner();
                            *guard = value;
                        }
                    }
                }
                Err(error) => {
                    error!("Error processing callback: {}", error);
                }
            }
        },
        (),
    )?;

    loop {
        thread::sleep(Duration::from_millis(1));
    }
}


/// Processes a single incoming MIDI message, updates the current message state,
/// and forwards the updated state to the given channel.
///
/// # Parameters
/// - `message`: A slice containing exactly three bytes of MIDI data conforming to MIDI standards
///   The slice is expected to have length ≥ 3; the first three bytes are used.
/// - `current_messages`: The current collection of active `MidiMessageData` entries.
/// - `tx`: A `Sender` over which the updated list of MIDI messages is sent.
///
/// # Returns
/// On success, returns the updated vector of `MidiMessageData` that represents
/// the current active MIDI state after applying the given `message`.
///
/// # Errors
/// Returns an error if:
/// - `message` does not have the correct number of bytes
/// - The raw `message` bytes cannot be converted into a valid `MidiMessageData`
///   instance (as determined by `MidiMessageData::new`), or
/// - Sending the updated MIDI message list on `tx` fails (for either of the
///   two send attempts).
pub(crate) fn process_callback(message: &[u8], current_messages: Vec<MidiMessageData>, tx: Sender<Vec<MidiMessageData>>) -> Result<Vec<MidiMessageData>, Box<dyn Error>> {
    if message.len() < 3 {
        return Err(format!("MIDI message too short: expected at least 3 bytes, got {}", message.len()).into());
    }

    let mut return_messages = current_messages;
    let midi_data = MidiMessageData::new(message[0], message[1], message[2])?;
    if midi_data.should_add_midi_message() {
        // Only add if note does not already exist
        if !return_messages
            .iter()
            .any(|x| x.data_byte1 == midi_data.data_byte1)
        {
            return_messages.push(midi_data.clone());
        }
    }

    if midi_data.should_remove_midi_message() {
        // Currently all MIDI channels will be "squished" in the
        // output to controller, so no need to filter by channel
        trace!("removing <- {:#04X?}", midi_data.data_byte1);
        return_messages.retain(|x| x.data_byte1 != midi_data.data_byte1);
    }

    // Send twice to ensure Gadget thread picks up the message
    tx.send(return_messages.clone()).map_err(|e| -> Box<dyn Error> { format!("failed to send MIDI messages (1st send): {e}").into() })?;
    tx.send(return_messages.clone()).map_err(|e| -> Box<dyn Error> { format!("failed to send MIDI messages (2nd send): {e}").into() })?;

    Ok(return_messages)
}

// Structure to store MIDI data packet
// Packet consists of 3 bytes
//     0 - Status byte + channel
//     1 - Data byte 1
//     2 - Data byte 2
//```
// Voice Message           Status Byte      Data Byte1          Data Byte2
// -------------           -----------   -----------------   -----------------
// Note off                      8x      Key number          Note Off velocity
// Note on                       9x      Key number          Note on velocity
// Polyphonic Key Pressure       Ax      Key number          Amount of pressure
// Control Change                Bx      Controller number   Controller value
// Program Change                Cx      Program number      None
// Channel Pressure              Dx      Pressure value      None
// Pitch Bend                    Ex      MSB                 LSB
// ```

#[derive(Clone)]
pub struct MidiMessageData {
    pub status_byte: MidiMessageTypes,
    pub data_byte1: u8,
    pub data_byte2: u8,
}

impl MidiMessageData {
    pub fn new(byte0: u8, byte1: u8, byte2: u8) -> Result<MidiMessageData, Box<dyn Error>> {
        let midi_type = match MidiMessageTypes::try_from(byte0 >> 4) {
            Ok(v) => v,
            Err(_) => return Err("Incorrect MidiMessageType".into()),
        };
        Ok(MidiMessageData {
            status_byte: midi_type,
            data_byte1: byte1,
            data_byte2: byte2,
        })
    }

    /// status is NoteOn AND velocity (data_byte2) IS NOT 0
    /// (0 is equivalent to NoteOff per MIDI standard)
    pub fn should_add_midi_message(&self) -> bool {
        self.status_byte == MidiMessageTypes::NoteOn
            && self.data_byte2 != 0x00u8
    }

    /// Status is NoteOff OR Status is NoteOn and 
    /// velocity (data_byte2) is 0
    /// (0 is equivalent to NoteOff per MIDI standard)
    pub fn should_remove_midi_message(&self) -> bool {
        (self.status_byte == MidiMessageTypes::NoteOn && self.data_byte2 == 0x00u8)
            || self.status_byte == MidiMessageTypes::NoteOff
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MidiMessageTypes {
    NoteOff = 0x8,
    NoteOn = 0x9,
    PolyphonicPressure = 0xA,
    ControlChange = 0xB,
    ProgramChange = 0xC,
    ChannelPressure = 0xD,
    PitchBend = 0xE,
}

impl TryFrom<u8> for MidiMessageTypes {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == MidiMessageTypes::NoteOff as u8 => Ok(MidiMessageTypes::NoteOff),
            x if x == MidiMessageTypes::NoteOn as u8 => Ok(MidiMessageTypes::NoteOn),
            x if x == MidiMessageTypes::PolyphonicPressure as u8 => {
                Ok(MidiMessageTypes::PolyphonicPressure)
            }
            x if x == MidiMessageTypes::ControlChange as u8 => Ok(MidiMessageTypes::ControlChange),
            x if x == MidiMessageTypes::ProgramChange as u8 => Ok(MidiMessageTypes::ProgramChange),
            x if x == MidiMessageTypes::ChannelPressure as u8 => {
                Ok(MidiMessageTypes::ChannelPressure)
            }
            x if x == MidiMessageTypes::PitchBend as u8 => Ok(MidiMessageTypes::PitchBend),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn should_add_midi_message_cases() {
        // NoteOn with non-zero velocity -> should add
        let m = MidiMessageData::new((MidiMessageTypes::NoteOn as u8) << 4, 0x3C, 0x40).unwrap();
        assert!(m.should_add_midi_message());
        assert!(!m.should_remove_midi_message());

        // NoteOn with zero velocity -> treated as NoteOff -> should not add, should remove
        let m = MidiMessageData::new((MidiMessageTypes::NoteOn as u8) << 4, 0x3C, 0x00).unwrap();
        assert!(!m.should_add_midi_message());
        assert!(m.should_remove_midi_message());

        // NoteOff -> should not add, should remove
        let m = MidiMessageData::new((MidiMessageTypes::NoteOff as u8) << 4, 0x3C, 0x00).unwrap();
        assert!(!m.should_add_midi_message());
        assert!(m.should_remove_midi_message());

        // Other message (ControlChange) -> should not add or remove
        let m = MidiMessageData::new((MidiMessageTypes::ControlChange as u8) << 4, 0x01, 0x7F).unwrap();
        assert!(!m.should_add_midi_message());
        assert!(!m.should_remove_midi_message());
    }

    #[test]
    fn process_callback_adds_message_and_sends() {
        let (tx, rx) = mpsc::channel();
        let persistent: Vec<MidiMessageData> = Vec::new();
        let msg = [(MidiMessageTypes::NoteOn as u8) << 4, 0x3C, 0x40];

        let res = process_callback(&msg, persistent, tx).expect("callback failed");
        // returned state should contain the note
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].data_byte1, 0x3C);

        // two sends were made; both should contain one element with the same note
        let first = rx.recv().expect("no first send");
        let second = rx.recv().expect("no second send");
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_eq!(first[0].data_byte1, 0x3C);
    }

    #[test]
    fn process_callback_does_not_add_duplicate_messages() {
        let (tx, rx) = mpsc::channel();
        // persistent already contains the note
        let existing = MidiMessageData::new((MidiMessageTypes::NoteOn as u8) << 4, 0x3C, 0x40).unwrap();
        let persistent = vec![existing.clone()];
        let msg = [(MidiMessageTypes::NoteOn as u8) << 4, 0x3C, 0x40];

        let res = process_callback(&msg, persistent, tx).expect("callback failed");
        // should not duplicate
        assert_eq!(res.len(), 1);
        let first = rx.recv().expect("no first send");
        let second = rx.recv().expect("no second send");
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
    }

    #[test]
    fn process_callback_remove_not_present_no_error() {
        let (tx, rx) = mpsc::channel();
        let persistent: Vec<MidiMessageData> = Vec::new();
        let msg = [(MidiMessageTypes::NoteOff as u8) << 4, 0x3C, 0x00];

        let res = process_callback(&msg, persistent, tx).expect("callback failed");
        assert!(res.is_empty());
        // two sends of empty vectors
        let first = rx.recv().expect("no first send");
        let second = rx.recv().expect("no second send");
        assert!(first.is_empty());
        assert!(second.is_empty());
    }

    #[test]
    fn process_callback_remove_present() {
        let (tx, rx) = mpsc::channel();
        let existing = MidiMessageData::new((MidiMessageTypes::NoteOn as u8) << 4, 0x3C, 0x40).unwrap();
        let persistent = vec![existing];
        let msg = [(MidiMessageTypes::NoteOff as u8) << 4, 0x3C, 0x00];

        let res = process_callback(&msg, persistent, tx).expect("callback failed");
        assert!(res.is_empty());
        let first = rx.recv().expect("no first send");
        let second = rx.recv().expect("no second send");
        assert!(first.is_empty());
        assert!(second.is_empty());
    }

    #[test]
    fn process_callback_persistence_across_iterations() {
        // First call: add note
        let (tx1, rx1) = mpsc::channel();
        let persistent: Vec<MidiMessageData> = Vec::new();
        let add_msg = [(MidiMessageTypes::NoteOn as u8) << 4, 0x3C, 0x40];
        let res1 = process_callback(&add_msg, persistent, tx1).expect("callback failed");
        assert_eq!(res1.len(), 1);
        // drain sends
        let _ = rx1.recv().unwrap();
        let _ = rx1.recv().unwrap();

        // Second call: no relevant midi message (ControlChange) but state should persist
        let (tx2, rx2) = mpsc::channel();
        let heartbeat = [(MidiMessageTypes::ControlChange as u8) << 4, 0x01, 0x7F];
        let res2 = process_callback(&heartbeat, res1.clone(), tx2).expect("callback failed");
        // res2 should still contain the previously added note
        assert_eq!(res2.len(), 1);
        let first = rx2.recv().expect("no first send");
        assert_eq!(first.len(), 1);
    }

    #[test]
    #[should_panic(expected = "Incorrect MidiMessageType")]
    fn process_callback_malformed_data_panics() {
        let (tx, _rx) = mpsc::channel();
        let persistent: Vec<MidiMessageData> = Vec::new();
        // byte0 high nibble 0x0 is not a valid MidiMessageTypes
        let bad = [0x00u8, 0x00u8, 0x00u8];
        // process_callback currently unwraps MidiMessageData::new(), so this will panic
        process_callback(&bad, persistent, tx).unwrap();
    }
}

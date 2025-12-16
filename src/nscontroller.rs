use crate::midi::MidiMessageData;
use lazy_static::lazy_static;
use log::error;
use std::collections::{HashMap, HashSet};
use std::error::Error;

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum Button {
    Y,
    X,
    B,
    A,
    R,
    ZR,

    Minus,
    Plus,
    RightStick,
    LeftStick,
    Home,
    Capture,

    DpadDown,
    DpadUp,
    DpadRight,
    DpadLeft,
    L,
    ZL,
}

lazy_static! {
    static ref KEYS_IN_BYTE1: HashSet<Button> = HashSet::from([
        Button::Y, Button::X, Button::B,
        Button::A, Button::R, Button::ZR,
    ]);
    static ref KEYS_IN_BYTE2: HashSet<Button> = HashSet::from([
        Button::Minus, Button::Plus, Button::RightStick,
        Button::LeftStick, Button::Home, Button::Capture,
    ]);
    static ref KEYS_IN_BYTE3: HashSet<Button> = HashSet::from([
        Button::DpadUp, Button::DpadDown, Button::DpadLeft,
        Button::DpadRight, Button::L, Button::ZL,
    ]);

    static ref KEY_OFFSET: HashMap<Button, u8> = {
        let mut m = HashMap::new();
        m.insert(Button::Y, 0);
        m.insert(Button::X, 1);
        m.insert(Button::B, 2);
        m.insert(Button::A, 3);
        m.insert(Button::R, 6);
        m.insert(Button::ZR, 7);

        m.insert(Button::Minus, 0);
        m.insert(Button::Plus, 1);
        m.insert(Button::RightStick, 2);
        m.insert(Button::LeftStick, 3);
        m.insert(Button::Home, 4);
        m.insert(Button::Capture, 5);

        m.insert(Button::DpadDown, 0);
        m.insert(Button::DpadUp, 1);
        m.insert(Button::DpadRight, 2);
        m.insert(Button::DpadLeft, 3);
        m.insert(Button::L, 6);
        m.insert(Button::ZL, 7);

        m
    };

    /// Taiko no Tatsujin configuration
    /// Controller Type 1
    /// R, L = Rim left and Rim right
    /// Down and B - Center Left and Right
    static ref MIDI_TO_INPUT: HashMap<u8, Vec<Button>> = {
        let mut m = HashMap::new();
        // Center with one hand
        m.insert(0x06u8, Vec::from([Button::DpadDown])); // F#2
        // Center with two hands
        // Actually this is one midi note which represents rimshot
        // But we need to press two buttons on virtual controller
        m.insert(0x28u8, Vec::from([Button::B, Button::DpadDown]));
        m.insert(0x25u8, Vec::from([Button::B, Button::DpadDown]));
        // Rim with one hand
        m.insert(0x30u8, Vec::from([Button::R]));
        // Rim with two hands
        m.insert(0x51u8, Vec::from([Button::R, Button::L]));
        m.insert(0x52u8, Vec::from([Button::R, Button::L]));
        m
    };
}

pub struct InputReport {
    pub report: [u8; 3],
}

/// Input report format
///  =========================================================================================================
/// | Bytes/Bits |     7    |    6    |     5      |     4      |     3     |     2     |     1     |    0    |
/// |    0x00    |                                             0x30                                           |
/// |    0x01    |                                           Timestamp                                        |
/// |    0x02    |                connection_info               |                 battery_level               |
/// |    0x03    |    ZR    |    R    | SR (right) | SL (right) |     A     |      B    |     X     |    Y    |
/// |    0x04    |   Grip   | (none)  |    Cap     |    Home    |   ThumbL  |   ThumbR  |     +     |    -    |
/// |    0x05    |    ZL    |    L      | SL (left)  | SR (left)  |    Left   |   Right   |    Up     |  Down   |
/// |    0x06    |                                          Analog [0]                                        |
/// |    0x07    |                                          Analog [1]                                        |
/// |    0x08    |                                          Analog [2]                                        |
/// |    0x09    |                                          Analog [3]                                        |
/// |    0x0a    |                                          Analog [4]                                        |
/// |    0x0b    |                                          Analog [5]                                        |
/// ==========================================================================================================
///
///
/// In this case we generate only bytes 0x03, 0x04, 0x05
/// To inject them to the actual input report from the controller
/// As result we need to implement pressing keys from Button enum only
///
/// example of full report
/// [
///     0x30, 0x00, 0x81, 0x00, 0x80, 0x00, 0xFB, 0xE7,
///     0x7F, 0xE1, 0xC7, 0x81, 0x01, 0xE9, 0xFC, 0x1E,
///     0x00, 0xD6, 0x0F, 0xEA, 0xFF, 0x04, 0x00, 0xFA,
///     0xFF, 0xEE, 0xFC, 0x28, 0x00, 0xD8, 0x0F, 0xEA,
///     0xFF, 0x04, 0x00, 0xFA, 0xFF, 0xF0, 0xFC, 0x2D,
///     0x00, 0xD4, 0x0F, 0xEC, 0xFF, 0x08, 0x00, 0xF9,
///     0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
/// ],
impl InputReport {
    pub fn new() -> InputReport {
        InputReport {
            report: [0x00, 0x80, 0x00],
        }
    }

    pub fn from(midi_data: &MidiMessageData) -> InputReport {
        let mut input_report = InputReport::new();
        let pressed_keys = match MIDI_TO_INPUT.get(&midi_data.data_byte1) {
            Some(value) => value,
            None => {
                error!(
                    "Unable to find note {:#04X?} in MIDI_TO_INPUT",
                    midi_data.data_byte1
                );
                return input_report;
            }
        };

        input_report.press(pressed_keys).unwrap();
        input_report
    }

    fn find_packet_position(&self, key: &Button) -> Result<usize, Box<dyn Error>> {
        let mut position: usize = 255;
        if KEYS_IN_BYTE1.contains(key) {
            position = 0;
        };
        if KEYS_IN_BYTE2.contains(key) {
            position = 1;
        };
        if KEYS_IN_BYTE3.contains(key) {
            position = 2;
        };
        if position == 255 {
            return Err(format!("Cannot find offset for {:?}", key).into());
        };
        Ok(position)
    }

    fn press_one(&mut self, key: &Button) -> Result<(), Box<dyn Error>> {
        let position = self.find_packet_position(key)?;
        match KEY_OFFSET.get(key) {
            Some(offset) => {
                self.report[position] |= 1 << offset;
            }
            None => return Err(format!("Cannot find offset for {:?}", key).into()),
        };
        Ok(())
    }

    pub fn press(&mut self, keys: &Vec<Button>) -> Result<(), Box<dyn Error>> {
        for key in keys {
            self.press_one(key)?;
        }
        Ok(())
    }
}

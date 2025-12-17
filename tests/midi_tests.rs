use midi_to_switch::midi::MidiMessageData;
use midi_to_switch::midi::MidiMessageTypes;

#[test]
fn try_from_valid_variants() {
    let mapping = vec![
        (0x8u8, MidiMessageTypes::NoteOff),
        (0x9u8, MidiMessageTypes::NoteOn),
        (0xAu8, MidiMessageTypes::PolyphonicPressure),
        (0xBu8, MidiMessageTypes::ControlChange),
        (0xCu8, MidiMessageTypes::ProgramChange),
        (0xDu8, MidiMessageTypes::ChannelPressure),
        (0xEu8, MidiMessageTypes::PitchBend),
    ];

    for (val, expected) in mapping {
        let got = MidiMessageTypes::try_from(val).expect("should convert");
        assert_eq!(got, expected);
    }
}

#[test]
fn try_from_invalid() {
    // 0x0 and 0xF are not valid message types in our enum
    assert!(MidiMessageTypes::try_from(0x0u8).is_err());
    assert!(MidiMessageTypes::try_from(0xFu8).is_err());
}

#[test]
fn midi_message_data_parsing_valid() {
    let byte0 = (0x9u8 << 4) | 0x3u8;
    let byte1 = 0x40u8;
    let byte2 = 0x7Fu8;

    let parsed = MidiMessageData::new(byte0, byte1, byte2).expect("parse should succeed");
    assert_eq!(parsed.channel, 0x3);
    assert_eq!(parsed.status_byte, MidiMessageTypes::NoteOn);
    assert_eq!(parsed.data_byte1, byte1);
    assert_eq!(parsed.data_byte2, byte2);
}

#[test]
fn midi_message_data_parsing_invalid_type() {
    let byte0 = (0x0u8 << 4) | 0x1u8;
    let res = MidiMessageData::new(byte0, 0x00, 0x00);
    assert!(res.is_err());
}
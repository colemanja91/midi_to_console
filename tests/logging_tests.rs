use log::{Level, LevelFilter};

#[test]
fn init_logger_sets_max_level_and_fails_second_time() {
    // First initialization should succeed
    let ok = midi_to_switch::logging::init_logger(LevelFilter::Info);
    assert!(ok.is_ok(), "first init_logger should succeed: {:?}", ok);

    // max level should be set to Info
    assert_eq!(log::max_level(), LevelFilter::Info);

    // log_enabled! reflects the set max level
    assert!(log::log_enabled!(Level::Info));
    assert!(!log::log_enabled!(Level::Debug));

    // Second initialization should fail because logger is already set
    let second = midi_to_switch::logging::init_logger(LevelFilter::Debug);
    assert!(second.is_err(), "second init_logger should return Err");
}

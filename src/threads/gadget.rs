use crate::device_file::DeviceFile;
use crate::midi::MidiMessageData;
use crate::nscontroller::InputReport;
use core::time;
use log::{debug, error, info, trace};
use std::error::Error;
use std::io::ErrorKind::WouldBlock;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;

/// Gadget control thread
///
/// Reads data from gadget and sends it to the controller
/// Receives data from the controller and writes it to the gadget
/// When controller receives init commands
/// For NS the sequence consists of 5 packets
///  [0x00, 0x00]
///  [0x00, 0x00]
///  [0x80, 0x05]
///  [0x00, 0x00]
///  [0x80, 0x01]
/// it starts to send input reports back
/// approx. 80 times per second
///
/// In this thread we re-send everything received from the controller to the USB gadget
/// However if there is input from midi device received from rx_midi
/// We replace the pressed keys in the input report with de keys we hit on the midi device
pub fn start_gadget(
    tx_controller: Sender<Vec<u8>>,
    rx_gadget: Receiver<Vec<u8>>,
    rx_midi: Receiver<Vec<MidiMessageData>>,
) -> Result<(), Box<dyn Error>> {
    info!("Starting gadget thread /dev/hidg0");
    let wait_ms = time::Duration::from_millis(5);
    let mut gadget_device = DeviceFile::new("/dev/hidg0", true).unwrap();

    loop {
        match rx_gadget.try_recv() {
            Ok(mut controller_data) => {
                trace!("rx_gadget -> gadget {:02X?}", controller_data);
                // We received data from controller
                // if it is input report
                // we need to inject inputs from midi device if we have any
                if controller_data[0] == 0x30 {
                    if let Ok(result) = rx_midi.try_recv() {
                        for midi_data in result {
                            debug!("midi_rx -> {:#04X?}", midi_data.data_byte1);
                            let input_report = InputReport::from(&midi_data);
                            controller_data[3] = input_report.report[0];
                            controller_data[4] = input_report.report[1];
                            controller_data[5] = input_report.report[2];
                        };
                    }
                }

                match gadget_device.write(controller_data) {
                    Ok(()) => {
                        trace!("gadget <-");
                    }
                    Err(error) => error!("Unable to write to gadget: {}", error),
                };
            }
            Err(TryRecvError::Empty) => {}
            Err(error) => {
                error!("Unable to receive data from rx_controller: {}", error);
            }
        };
        match gadget_device.read() {
            Ok(value) => {
                trace!("gadget -> {:02X?}", value);
                match tx_controller.send(value) {
                    Ok(()) => {}
                    Err(error) => {
                        panic!("Cannot send to tx_controller {}", error);
                    }
                };
            }
            Err(error) => {
                // WouldBlock is expected behavior
                // usually meaning there is no data in the device yet
                if error.kind() != WouldBlock {
                    error!("Gadget read error: {}", error);
                }
            }
        };
        thread::sleep(wait_ms);
    }
}

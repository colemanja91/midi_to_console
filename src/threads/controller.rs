use crate::device_file::DeviceFile;
use core::time;
use log::{error, info, trace};
use std::error::Error;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;

/// Gamepad control thread
/// Reads data from gamepad and sends it to the gadget api device
/// Receives data from gadget api device and sends it to the controller
pub fn start_controller(
    tx_gadget: Sender<Vec<u8>>,
    rx_controller: Receiver<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    info!("Starting controller thread /dev/hidraw0");

    let mut controller = DeviceFile::new("/dev/hidraw0", true).unwrap();

    let wait_ms = time::Duration::from_millis(5);

    loop {
        match rx_controller.try_recv() {
            Ok(received) => {
                trace!("rx_controller -> controller {:02X?}", received);
                match controller.write(received) {
                    Ok(_) => {
                        trace!("conroller <-");
                    }
                    Err(error) => error!("Unable to write to controller: {}", error),
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(error) => {
                error!("rx_gadget: {:?}", error)
            }
        };

        if let Ok(buf) = controller.read() {
            trace!("controller ->");
            match tx_gadget.send(buf) {
                Ok(()) => {
                    trace!("tx_gadget <- controller");
                }
                Err(error) => {
                    panic!("Cannot send to tx_gadget {}", error);
                }
            };
        };
        thread::sleep(wait_ms);
    }
}

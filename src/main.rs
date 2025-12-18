extern crate core;

use crate::logging::init_logger;
use crate::midi::{process_signals, MidiMessageData};
use crate::threads::controller::start_controller;
use crate::threads::gadget::start_gadget;
use core::time;
use log::LevelFilter;
use std::fs::OpenOptions;
use std::process::Command;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

mod device_file;
mod logging;
mod midi;
mod nscontroller;
mod threads {
    pub mod gadget;
    pub mod controller;
}

fn reconnect_controller() {
    // Disconnect gadget from USB OTG port
    // echo > /sys/kernel/config/usb_gadget/procon/UDC
    {
        let gadget_file = OpenOptions::new()
            .write(true)
            .open("/sys/kernel/config/usb_gadget/procon/UDC")
            .unwrap();
        let mut command_disconnect = Command::new("echo").stdout(gadget_file).spawn().unwrap();
        command_disconnect.wait().unwrap();
    }
    // Connect gadget to USB OTG port
    // ls /sys/class/udc > /sys/kernel/config/usb_gadget/procon/UDC
    {
        let gadget_file = OpenOptions::new()
            .write(true)
            .open("/sys/kernel/config/usb_gadget/procon/UDC")
            .unwrap();
        let mut command_connect = Command::new("ls")
            .arg("/sys/class/udc")
            .stdout(gadget_file)
            .spawn()
            .unwrap();
        command_connect.wait().unwrap();
    }
    let wait_ms = time::Duration::from_millis(500);
    thread::sleep(wait_ms);
}

fn main() {
    init_logger(LevelFilter::Info).unwrap();
    // reconnect controller for host to send
    // init packets to the game controller
    reconnect_controller();

    // channels to control communication between gamepads
    let (tx_controller, rx_controller): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let (tx_gadget, rx_gadget): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    // channel to receive MidiMessageData
    let (tx_midi, rx_midi): (Sender<Vec<MidiMessageData>>, Receiver<Vec<MidiMessageData>>) = mpsc::channel();

    // thread to process usb gadget data via gadgetfs
    thread::Builder::new()
        .name(String::from("gadget"))
        .spawn(move || start_gadget(tx_controller.clone(), rx_gadget, rx_midi).unwrap())
        .unwrap();
    // thread to process usb controller
    thread::Builder::new()
        .name(String::from("controller"))
        .spawn(move || start_controller(tx_gadget.clone(), rx_controller).unwrap())
        .unwrap();

    process_signals(1, tx_midi).unwrap();
}

use libc::O_NONBLOCK;
use log::error;
use std::fs::{File, OpenOptions};
use std::io::{Error, Read, Write};
use std::os::unix::fs::OpenOptionsExt;

pub struct DeviceFile {
    fp: File,
}

impl DeviceFile {
    pub fn new(dev_path: &str, non_block: bool) -> Result<DeviceFile, Error> {
        let fp = OpenOptions::new()
            .custom_flags(if non_block { O_NONBLOCK } else { 0 })
            .read(true)
            .write(true)
            .open(dev_path)?;
        Ok(DeviceFile { fp })
    }

    pub fn write(&mut self, data: Vec<u8>) -> Result<(), Error> {
        if let Err(e) = self.fp.write_all(data.as_ref()) {
            error!("Unable to write to {:?}: {}", self.fp, e);
        }
        Ok(())
    }

    pub fn read(&mut self) -> Result<Vec<u8>, Error> {
        let mut buf = vec![0u8; 64];
        self.fp.read_exact(&mut buf)?;
        Ok(buf)
    }
}

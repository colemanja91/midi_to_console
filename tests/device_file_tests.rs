use std::fs::{OpenOptions, remove_file};
use std::io::{Write, Seek, SeekFrom};
use std::path::PathBuf;

fn temp_path(name: &str) -> PathBuf {
	let mut p = std::env::temp_dir();
	let ts = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	p.push(format!("midi_to_switch_test_{}_{}.tmp", name, ts));
	p
}

fn create_file_with_contents(path: &PathBuf, contents: &[u8]) {
	let mut f = OpenOptions::new()
		.create(true)
		.read(true)
		.write(true)
		.open(path)
		.expect("create temp file");
	f.write_all(contents).expect("write initial data");
	f.seek(SeekFrom::Start(0)).expect("seek to start");
}

#[test]
fn new_opens_file() {
	let path = temp_path("new");
	create_file_with_contents(&path, &vec![0u8; 64]);

	// Should open successfully with and without O_NONBLOCK
	let _dev = midi_to_switch::device_file::DeviceFile::new(path.to_str().unwrap(), false)
		.expect("DeviceFile::new failed");

	let _dev_nb = midi_to_switch::device_file::DeviceFile::new(path.to_str().unwrap(), true)
		.expect("DeviceFile::new with non-block failed");

	remove_file(path).expect("cleanup");
}

#[test]
fn read_returns_64_bytes() {
	let path = temp_path("read");
	let mut initial = Vec::with_capacity(64);
	for i in 0..64u8 {
		initial.push(i);
	}
	create_file_with_contents(&path, &initial);

	let mut dev = midi_to_switch::device_file::DeviceFile::new(path.to_str().unwrap(), false)
		.expect("open device file");

	let buf = dev.read().expect("read failed");
	assert_eq!(buf.len(), 64);
	assert_eq!(buf, initial);

	remove_file(path).expect("cleanup");
}

#[test]
fn write_overwrites_file() {
	let path = temp_path("write");
	create_file_with_contents(&path, &vec![0u8; 64]);

	let mut dev = midi_to_switch::device_file::DeviceFile::new(path.to_str().unwrap(), false)
		.expect("open device file");

	// write a short sequence and verify it's present on disk
	dev.write(vec![1u8, 2, 3]).expect("write failed");

	let on_disk = std::fs::read(&path).expect("read file");
	assert!(on_disk.len() >= 3);
	assert_eq!(&on_disk[0..3], &[1u8, 2, 3]);

	remove_file(path).expect("cleanup");
}

# HOSTING
The primary working repository for this project is https://codeberg.org/colemanja91/midi_to_console

The GitHub repository will remain a mirror for some time: https://github.com/colemanja91/midi_to_console

# About

Forked from https://github.com/Shtsh/midi_to_console

Simple middleware between nintendo switch and controller 
allowing injecting midi input to send input to the console.

Currently setup to use a static mapping of MIDI pitches to Switch Controller buttons.

# Requirements
* Nintendo Switch
* Raspberry PI (I used 4B but any with USB OTG will work)
* Nintendo switch controller

To be able to act as a fake controller the host running the software has to have hardware support for USB client mode.
USB RSP (or OTG) will work.

For some Raspberry PIs the power provided by Nintendo switch might be not enough 
so some sort of UPS HAT or any type of additional power is recommended.

The application uses linux-specific gadget api to emulate controller so no cross-platform is expected.

# Installation on Raspberry PI 
Instruction assumes there is Raspberry Pi OS based on Debian 11 has already been installed.

## Allowing OTP on USB-C port
Add to the end of /boot/firmware/config.txt
```
dtoverlay=dwc2
```

Add to /boot/firmware/cmdline.txt **after** `rootwait`
```
modules-load=dwc2,libcomposite
```

## Build and install the device
Install rust following this doc: https://www.rust-lang.org/tools/install

Build and install the package
```
cargo install cargo-deb
cargo deb
sudo apt install ./target/debian/midi-to-switch_0.1.0_arm64.deb
```
After reboot everything should be up an running

**TODO**

* Add troubleshooting steps based on https://askubuntu.com/questions/1493517/how-to-solve-could-not-execute-systemctl-at-usr-bin-deb-systemd-invoke-line-1
* Fix script for the following (temp fix possibly powering not via USB-C):

```sh
Dec 18 20:26:08 mushroompi create_gadget_device.sh[1727]: /usr/sbin/create_gadget_device.sh: line 26: echo: write error: Device or resource busy
Dec 18 20:26:08 mushroompi create_gadget_device.sh[1727]: /usr/sbin/create_gadget_device.sh: line 27: echo: write error: Device or resource busy
Dec 18 20:26:08 mushroompi create_gadget_device.sh[1727]: /usr/sbin/create_gadget_device.sh: line 28: echo: write error: Device or resource busy
Dec 18 20:26:08 mushroompi create_gadget_device.sh[1734]: /usr/sbin/create_gadget_device.sh: line 29: xxd: command not found
Dec 18 20:26:08 mushroompi create_gadget_device.sh[1735]: ln: failed to create symbolic link 'configs/c.1/hid.usb0': File exists
Dec 18 20:26:08 mushroompi create_gadget_device.sh[1736]: ls: write error: Device or resource busy
```


# Acknowledgements

* Used NS protocol analysis https://www.mzyy94.com/blog/2020/03/20/nintendo-switch-pro-controller-usb-gadget/
* Thanks Shtsh for the original work on this repo

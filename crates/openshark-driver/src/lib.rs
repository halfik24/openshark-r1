//! Linux driver for the Attack Shark R1 mouse (2.4G wireless + wired).
//!
//! Speaks the vendor protocol over USB:
//! interface 2, control transfers (`SET_REPORT`) for configuration and an
//! interrupt IN endpoint (`0x83`) for acknowledgements and battery telemetry.

use std::time::Duration;

use rusb::{DeviceHandle, GlobalContext};

pub mod dpi_codes;
pub mod protocol;

pub use protocol::{Config, PollingRate, FACTORY_BUTTONS};
// Для сопоставления кодов usb-ошибок на стороне приложения.
pub use rusb;

pub const VENDOR_ID: u16 = 0x1d57;
pub const PRODUCT_ID_WIRELESS: u16 = 0xfa60;
pub const PRODUCT_ID_WIRED: u16 = 0xfa61;

/// Vendor interface used for configuration and telemetry.
const INTERFACE: u8 = 2;
/// Interrupt IN endpoint carrying ACKs and battery reports.
const BATTERY_ENDPOINT: u8 = 0x83;

const TRANSFER_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Attack Shark R1 not found (vid=0x{VENDOR_ID:04x})")]
    NotFound,
    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),
    #[error("short battery report ({0} bytes)")]
    ShortReport(usize),
    #[error("device did not acknowledge the configuration report")]
    NoAck,
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

/// An open, claimed Attack Shark R1 on interface 2.
///
/// The kernel driver bound to the interface (if any) is detached on [`Mouse::open`]
/// and re-attached when the handle is dropped.
pub struct Mouse {
    handle: DeviceHandle<GlobalContext>,
    wired: bool,
    detached: bool,
}

impl Mouse {
    /// Find and claim the mouse.
    pub fn open() -> Result<Self, Error> {
        let mut target: Option<(rusb::Device<GlobalContext>, bool)> = None;

        for device in rusb::devices()?.iter() {
            let desc = device.device_descriptor()?;
            if desc.vendor_id() != VENDOR_ID {
                continue;
            }
            if matches!(
                desc.product_id(),
                PRODUCT_ID_WIRELESS | PRODUCT_ID_WIRED
            ) {
                target = Some((device, desc.product_id() == PRODUCT_ID_WIRED));
                break;
            }
        }

        let (device, wired) = target.ok_or(Error::NotFound)?;
        let handle = device.open()?;

        // The kernel driver may hold interface 2; detach it for the duration of
        // our access and put it back on drop.
        let detached = handle.kernel_driver_active(INTERFACE)?;
        if detached {
            handle.detach_kernel_driver(INTERFACE)?;
        }
        handle.claim_interface(INTERFACE)?;

        Ok(Self {
            handle,
            wired,
            detached,
        })
    }

    /// Whether the mouse is connected over USB cable (no battery telemetry).
    pub fn is_wired(&self) -> bool {
        self.wired
    }

    /// Read the current battery charge as a percentage (0-100).
    ///
    /// Only meaningful on the wireless receiver; wired mode always reports 0.
    pub fn battery_percent(&self) -> Result<u8, Error> {
        let mut buf = [0u8; 64];
        let read = self
            .handle
            .read_interrupt(BATTERY_ENDPOINT, &mut buf, TRANSFER_TIMEOUT)?;

        if read < 5 {
            return Err(Error::ShortReport(read));
        }

        // Charge is encoded in tenths: byte 4 holds `percent / 10`.
        Ok(buf[4].saturating_mul(10).min(100))
    }
}

impl Drop for Mouse {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(INTERFACE);
        if self.detached {
            let _ = self.handle.attach_kernel_driver(INTERFACE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // touches real hardware: run with `cargo test -- --ignored`
    fn battery_reads_from_real_device() {
        // Skipped automatically when the mouse is not plugged in.
        let Ok(mouse) = Mouse::open() else {
            eprintln!("device not present, skipping");
            return;
        };
        if mouse.is_wired() {
            return;
        }
        let percent = mouse.battery_percent().expect("battery read");
        assert!(percent <= 100);
        assert_eq!(percent % 10, 0, "charge granularity is 10%");
    }
}

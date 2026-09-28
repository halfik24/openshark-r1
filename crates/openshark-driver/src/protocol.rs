//! Configuration protocol for the Attack Shark R1.
//!
//! Ported byte-for-byte from the Odin reference implementation
//! (`main.odin` / `dpi.odin` of xb-bx/attack-shark-r1-driver):
//! three `SET_REPORT` control transfers on interface 2 (report ids
//! `0x06` polling rate, `0x05` timers, `0x04` DPI), each acknowledged by
//! the mouse with byte `0x50` on the interrupt endpoint `0x83`.

use serde::{Deserialize, Serialize};

use crate::dpi_codes::DPI_CODES;
use crate::Error;
use crate::Mouse;

/// Byte the mouse reports on `0x83` after accepting a configuration report.
const ACK: u8 = 0x50;
/// The reference driver re-sends until acknowledged; we bound the attempts.
const ACK_ATTEMPTS: u8 = 3;

/// USB polling rate, encoded as a little-endian `u16` in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollingRate {
    Hz125 = 0xf708,
    Hz250 = 0xfb04,
    Hz500 = 0xfd02,
    Hz1000 = 0xfe01,
}

impl PollingRate {
    pub fn from_hz(hz: u16) -> Option<Self> {
        match hz {
            125 => Some(Self::Hz125),
            250 => Some(Self::Hz250),
            500 => Some(Self::Hz500),
            1000 => Some(Self::Hz1000),
            _ => None,
        }
    }

    pub fn hz(self) -> u16 {
        match self {
            Self::Hz125 => 125,
            Self::Hz250 => 250,
            Self::Hz500 => 500,
            Self::Hz1000 => 1000,
        }
    }
}

/// Full user configuration, serialised to disk as JSON by the GUI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// One of 125, 250, 500, 1000.
    pub polling_rate: u16,
    /// Exactly six values, each 100..=18000 in steps of 100.
    pub dpis: Vec<u16>,
    /// Selected stage, 1..=6.
    pub active_dpi: u8,
    /// Idle time before sleep, seconds (0.5..=30).
    pub sleep_time: f64,
    /// Deep sleep delay, seconds (1..=60).
    pub deep_sleep_time: u8,
    /// Key response time, ms, even (4..=50).
    pub key_response_time: u8,
    pub ripple_control: bool,
    pub angle_snap: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            polling_rate: 125,
            dpis: vec![800, 1600, 3200, 4000, 5000, 12000],
            active_dpi: 3,
            sleep_time: 6.0,
            deep_sleep_time: 12,
            key_response_time: 4,
            ripple_control: false,
            angle_snap: false,
        }
    }
}

impl Config {
    /// Same bounds the reference driver enforces when loading its INI.
    pub fn validate(&self) -> Result<(), String> {
        if PollingRate::from_hz(self.polling_rate).is_none() {
            return Err(format!(
                "polling rate must be 125, 250, 500 or 1000 (got {})",
                self.polling_rate
            ));
        }
        if self.dpis.len() != 6 {
            return Err(format!("expected 6 DPI stages, got {}", self.dpis.len()));
        }
        for (i, &dpi) in self.dpis.iter().enumerate() {
            if !(100..=18000).contains(&dpi) || dpi % 100 != 0 {
                return Err(format!(
                    "DPI stage {} must be 100..18000 in steps of 100 (got {})",
                    i + 1,
                    dpi
                ));
            }
        }
        if !(1..=6).contains(&self.active_dpi) {
            return Err(format!("active DPI stage must be 1..6 (got {})", self.active_dpi));
        }
        if !(0.5..=30.0).contains(&self.sleep_time) {
            return Err(format!("sleep time must be 0.5..30 s (got {})", self.sleep_time));
        }
        if !(1..=60).contains(&self.deep_sleep_time) {
            return Err(format!(
                "deep sleep must be 1..60 s (got {})",
                self.deep_sleep_time
            ));
        }
        if self.key_response_time < 4
            || self.key_response_time > 50
            || !self.key_response_time.is_multiple_of(2)
        {
            return Err(format!(
                "key response time must be an even number in 4..50 ms (got {})",
                self.key_response_time
            ));
        }
        Ok(())
    }
}

/// Report `0x06` — polling rate at bytes 3..5, little-endian.
fn polling_payload(rate: PollingRate) -> [u8; 9] {
    let mut p = [0x6, 0x9, 0x1, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0];
    p[3..5].copy_from_slice(&(rate as u16).to_le_bytes());
    p
}

/// Factory button assignment: firmware action byte per slot.
///
/// Slot order: left, right, middle, -, -, DPI cycle, forward, backward,
/// 9..=16 disabled, scroll up, scroll down.
pub const FACTORY_BUTTONS: [u8; 18] = [
    0x02, 0x03, 0x04, 0x01, 0x01, 0x0d, 0x06, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x09, 0x0a,
];

/// Report `0x08` — button mapping: profile 1, 18 x 3-byte slots, checksum.
fn button_map_payload(slots: &[u8; 18]) -> [u8; 59] {
    let mut p = [0u8; 59];
    p[0] = 0x08;
    p[1] = 0x3b;
    p[2] = 0x01;
    for (i, &action) in slots.iter().enumerate() {
        let o = 3 + i * 3;
        p[o] = action;
        p[o + 1] = 0x00;
        p[o + 2] = 0x00;
    }
    // Sum of bytes 2..58, minus one, stored in the last byte.
    let sum = p[2..58].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    p[58] = sum.wrapping_sub(1);
    p
}

/// Report `0x05` — sleep/deep-sleep/key-response timers and their checksum.
fn times_payload(sleep_time: f64, deep_sleep: u8, key_resp: u8) -> [u8; 15] {
    let mut p = [
        0x5, 0xf, 0x1, 0x0, 0x03, 0x18, 0x0, 0x0, 0xff, 0x4, 0x2, 0x1, 0x20, 0x0, 0x0,
    ];
    p[4] = 0x03 | (deep_sleep & 0xF0);
    p[5] = 0x08 | ((deep_sleep & 0x0F) << 4);
    p[9] = (sleep_time * 2.0) as u8;
    p[10] = key_resp / 2;
    // ((low nibble + high nibble) << 4) + 0x0a + the two encoded timers.
    let nibbles = (deep_sleep & 0x0F).wrapping_add((deep_sleep & 0xF0) >> 4);
    p[12] = (nibbles << 4)
        .wrapping_add(0x0a)
        .wrapping_add(p[9])
        .wrapping_add(p[10]);
    p
}

/// Report `0x04` — six DPI stages, flags, active stage and checksum.
fn dpis_payload(
    dpis: &[u16; 6],
    active_dpi: u8,
    ripple_control: bool,
    angle_snap: bool,
) -> [u8; 56] {
    let mut p: [u8; 56] = [
        0x4, 0x38, 0x1, 0x0, 0x0, 0x3f, 0x0, 0x0,
        0x2, 0x2, 0x2, 0x2, 0x2, 0x2, 0x0, 0x0,
        0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
        0x1, 0xff, 0x0, 0x0, 0x0, 0xff, 0x0, 0x0,
        0x0, 0xff, 0xff, 0xff, 0x0, 0x0, 0xff, 0xff,
        0xff, 0x0, 0xff, 0xff, 0x40, 0x0, 0xff, 0xff,
        0xff, 0x2, 0xd, 0x75, 0x0, 0x0, 0x0, 0x0,
    ];
    let mut checksum: u16 = 0x0d75;
    let mut big_mask: u8 = 0;

    for (i, &dpi) in dpis.iter().enumerate() {
        let code = dpi_code(dpi);
        p[i + 8] = code;
        checksum = checksum.wrapping_add(code as u16);

        let in_band = u8::from((10100..=12000).contains(&dpi));
        p[i + 16] = in_band;
        checksum = checksum.wrapping_add(in_band as u16);

        if dpi > 12000 {
            big_mask |= 1 << i;
        }
    }
    p[6] = big_mask;
    p[7] = big_mask;
    checksum = checksum.wrapping_add(big_mask as u16 * 2);

    p[24] = active_dpi;
    checksum = checksum.wrapping_add(active_dpi as u16 - 1);

    if ripple_control {
        checksum = checksum.wrapping_add(1);
        p[4] = 1;
    }
    if angle_snap {
        checksum = checksum.wrapping_add(1);
        p[3] = 1;
    }

    p[50..52].copy_from_slice(&checksum.to_be_bytes());
    p
}

/// Wire code for a DPI value (steps of 100 in 100..=18000).
fn dpi_code(dpi: u16) -> u8 {
    DPI_CODES[(dpi / 100 - 1) as usize]
}

impl Mouse {
    /// Send a configuration report and wait for the `0x50` ACK.
    fn send_report(&self, payload: &[u8]) -> Result<(), Error> {
        // wValue high byte 0x03 = HID feature report, low byte = report id.
        let value = 0x0300 | payload[0] as u16;
        for _ in 0..ACK_ATTEMPTS {
            self.handle.write_control(
                0x21,
                0x09,
                value,
                crate::INTERFACE as u16,
                payload,
                crate::TRANSFER_TIMEOUT,
            )?;
            if self.wired {
                return Ok(());
            }
            let mut buf = [0u8; 64];
            match self.handle.read_interrupt(
                crate::BATTERY_ENDPOINT,
                &mut buf,
                crate::TRANSFER_TIMEOUT,
            ) {
                Ok(n) if n >= 3 && buf[2] == ACK => return Ok(()),
                // Timeout or a battery report: re-send and keep waiting.
                Ok(_) | Err(rusb::Error::Timeout) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Err(Error::NoAck)
    }

    pub fn set_polling_rate(&self, rate: PollingRate) -> Result<(), Error> {
        self.send_report(&polling_payload(rate))
    }

    pub fn set_times(&self, sleep_time: f64, deep_sleep: u8, key_resp: u8) -> Result<(), Error> {
        self.send_report(&times_payload(sleep_time, deep_sleep, key_resp))
    }

    pub fn set_dpis(
        &self,
        dpis: &[u16; 6],
        active_dpi: u8,
        ripple_control: bool,
        angle_snap: bool,
    ) -> Result<(), Error> {
        self.send_report(&dpis_payload(dpis, active_dpi, ripple_control, angle_snap))
    }

    /// Write the button-mapping table (feature report `0x08`, 59 bytes).
    ///
    /// `slots` holds the 18 firmware action bytes; modifiers/usage bytes are
    /// written as 0 and the checksum is computed automatically.
    pub fn set_button_map(&self, slots: &[u8; 18]) -> Result<(), Error> {
        self.send_report(&button_map_payload(slots))
    }

    /// Restore every button to the factory assignment (left/right/middle,
    /// side forward/back, DPI cycle, scroll up/down; the rest disabled).
    ///
    /// This is the fix for a mouse whose buttons were remapped to nothing.
    pub fn reset_buttons(&self) -> Result<(), Error> {
        self.set_button_map(&FACTORY_BUTTONS)
    }

    /// Apply a validated [`Config`] in the same order as the reference driver.
    pub fn apply_config(&self, config: &Config) -> Result<(), Error> {
        config.validate().map_err(Error::InvalidConfig)?;

        let rate = PollingRate::from_hz(config.polling_rate)
            .ok_or_else(|| Error::InvalidConfig(format!("bad polling rate {}", config.polling_rate)))?;
        self.set_polling_rate(rate)?;
        self.set_times(config.sleep_time, config.deep_sleep_time, config.key_response_time)?;

        let dpis: [u16; 6] = config
            .dpis
            .as_slice()
            .try_into()
            .map_err(|_| Error::InvalidConfig("expected 6 DPI stages".into()))?;
        self.set_dpis(&dpis, config.active_dpi, config.ripple_control, config.angle_snap)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The untouched template of report 0x04 (as in `main.odin`).
    const DPI_TEMPLATE: [u8; 56] = [
        0x4, 0x38, 0x1, 0x0, 0x0, 0x3f, 0x0, 0x0,
        0x2, 0x2, 0x2, 0x2, 0x2, 0x2, 0x0, 0x0,
        0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
        0x1, 0xff, 0x0, 0x0, 0x0, 0xff, 0x0, 0x0,
        0x0, 0xff, 0xff, 0xff, 0x0, 0x0, 0xff, 0xff,
        0xff, 0x0, 0xff, 0xff, 0x40, 0x0, 0xff, 0xff,
        0xff, 0x2, 0xd, 0x75, 0x0, 0x0, 0x0, 0x0,
    ];

    #[test]
    fn polling_payloads_match_reference() {
        assert_eq!(
            polling_payload(PollingRate::Hz125),
            [0x6, 0x9, 0x1, 0x08, 0xf7, 0x0, 0x0, 0x0, 0x0]
        );
        assert_eq!(
            polling_payload(PollingRate::Hz1000),
            [0x6, 0x9, 0x1, 0x01, 0xfe, 0x0, 0x0, 0x0, 0x0]
        );
    }

    #[test]
    fn times_payload_matches_reference_template() {
        // Template bytes of report 0x05 correspond to sleep=2, deep=1, key=4.
        assert_eq!(
            times_payload(2.0, 1, 4),
            [0x5, 0xf, 0x1, 0x0, 0x03, 0x18, 0x0, 0x0, 0xff, 0x4, 0x2, 0x1, 0x20, 0x0, 0x0]
        );
    }

    #[test]
    fn times_payload_encodes_default_ini() {
        // sleep=6 -> 12, deep=12 -> 0x03/0xC8, key=4 -> 2, checksum 0xD8.
        assert_eq!(
            times_payload(6.0, 12, 4),
            [0x5, 0xf, 0x1, 0x0, 0x03, 0xc8, 0x0, 0x0, 0xff, 0x0c, 0x2, 0x1, 0xd8, 0x0, 0x0]
        );
    }

    #[test]
    fn dpis_payload_defaults_only_change_checksum() {
        // All stages at 100 DPI (code 0x02), stage 1 active: the template
        // already holds these values, so only the checksum moves 0x0d75 -> 0x0d81.
        let mut expected = DPI_TEMPLATE;
        expected[50..52].copy_from_slice(&[0x0d, 0x81]);
        assert_eq!(dpis_payload(&[100; 6], 1, false, false), expected);
    }

    #[test]
    fn dpis_payload_tracks_flags_and_active_stage() {
        let dpis = [800, 1600, 3200, 4000, 5000, 12000];
        let p = dpis_payload(&dpis, 3, true, true);

        // 12000 sits in the flagged band but not above 12000.
        assert_eq!(&p[8..14], &[0x12, 0x25, 0x4b, 0x5e, 0x75, 0x8d]);
        assert_eq!(&p[16..22], &[0, 0, 0, 0, 0, 1]);
        assert_eq!(p[6], 0);
        assert_eq!(p[7], 0);
        assert_eq!(p[24], 3);
        assert_eq!(p[3], 1, "angle snap flag");
        assert_eq!(p[4], 1, "ripple flag");

        let expected: u16 = 0x0d75
            + 0x12
            + 0x25
            + 0x4b
            + 0x5e
            + 0x75
            + 0x8d
            + 1
            + 2
            + 1
            + 1;
        assert_eq!(&p[50..52], &expected.to_be_bytes());
    }

    #[test]
    fn dpis_payload_sets_above_12k_mask() {
        let p = dpis_payload(&[13000, 13000, 100, 100, 100, 100], 6, false, false);
        assert_eq!(p[6], 0b0000_0011);
        assert_eq!(p[7], 0b0000_0011);
        assert_eq!(p[24], 6);
        assert_eq!(&p[16..22], &[0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn dpi_codes_match_reference_spots() {
        assert_eq!(dpi_code(100), 0x02);
        assert_eq!(dpi_code(1000), 0x17);
        assert_eq!(dpi_code(3200), 0x4b);
        assert_eq!(dpi_code(10000), 0xeb);
        assert_eq!(dpi_code(18000), 0xd3);
    }

    #[test]
    fn default_config_is_valid() {
        Config::default().validate().expect("default config");
    }

    #[test]
    fn config_validation_rejects_bad_values() {
        let c = Config {
            polling_rate: 200,
            ..Default::default()
        };
        assert!(c.validate().is_err());

        let mut c = Config::default();
        c.dpis.pop();
        assert!(c.validate().is_err());

        let mut c = Config::default();
        c.dpis[0] = 150;
        assert!(c.validate().is_err());

        let c = Config {
            active_dpi: 7,
            ..Default::default()
        };
        assert!(c.validate().is_err());

        let c = Config {
            sleep_time: 0.1,
            ..Default::default()
        };
        assert!(c.validate().is_err());

        let c = Config {
            key_response_time: 5,
            ..Default::default()
        };
        assert!(c.validate().is_err());
    }

    #[test]
    fn config_roundtrips_through_json() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    #[ignore] // touches real hardware: run with `cargo test -- --ignored`
    fn hardware_apply_config() {
        // Only runs with the mouse plugged in; ignored by default so that a
        // plain `cargo test` never rewrites the device settings.
        let Ok(mouse) = Mouse::open() else {
            eprintln!("device not present, skipping");
            return;
        };
        mouse
            .apply_config(&Config::default())
            .expect("apply config to real device");
    }
}

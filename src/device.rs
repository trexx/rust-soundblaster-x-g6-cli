use std::io::Write;

use anyhow::{anyhow, bail, Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};

use crate::spec::HidFrame;

pub const G6_VENDOR_ID: u16 = 0x041e;
pub const G6_PRODUCT_ID: u16 = 0x3256;
/// The G6 exposes two HID interfaces; only this one accepts control frames.
pub const G6_HID_INTERFACE: i32 = 4;

/// hidapi requires a leading report-ID byte (0x00 for this device) in front of the 64-byte payload.
const REPORT_LEN: usize = HidFrame::LEN + 1;

/// Destination for HID frames: the real device, or a dry-run printer.
pub trait FrameSink {
    fn send(&self, frames: &[HidFrame]) -> Result<()>;
}

/// The G6 control interface in `api`'s current device list, if the device is plugged in.
pub fn find(api: &HidApi) -> Option<&DeviceInfo> {
    api.device_list().find(|d| {
        d.vendor_id() == G6_VENDOR_ID
            && d.product_id() == G6_PRODUCT_ID
            && d.interface_number() == G6_HID_INTERFACE
    })
}

/// The physical G6, opened over hidapi.
pub struct G6 {
    device: HidDevice,
    debug: bool,
}

impl G6 {
    /// Enumerate HID devices and open the G6.
    pub fn open() -> Result<Self> {
        let api = HidApi::new().context("Failed to initialize HID API")?;
        Self::open_with(&api)
    }

    /// Open the G6 from an already enumerated `HidApi`.
    pub fn open_with(api: &HidApi) -> Result<Self> {
        let info = find(api).ok_or_else(|| {
            anyhow!(
                "SoundBlaster X G6 not found (VID={G6_VENDOR_ID:#06x} PID={G6_PRODUCT_ID:#06x} \
                 interface={G6_HID_INTERFACE}).\n\
                 Is the device plugged in? Check Device Manager → Human Interface Devices."
            )
        })?;
        let device = info
            .open_device(api)
            .context("Failed to open G6 HID interface")?;
        Ok(Self {
            device,
            debug: false,
        })
    }

    /// Echo every frame sent and every response received to stderr.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Read whatever the device has queued, without blocking.
    fn drain_responses(&self) {
        let mut resp = [0u8; HidFrame::LEN];
        loop {
            match self.device.read_timeout(&mut resp, 0) {
                Ok(n) if n > 0 => {
                    if self.debug {
                        eprintln!("Response({n}): {:02x?}", &resp[..n]);
                    }
                }
                _ => break,
            }
        }
    }
}

impl FrameSink for G6 {
    fn send(&self, frames: &[HidFrame]) -> Result<()> {
        for frame in frames {
            if self.debug {
                eprintln!("{}", frame.to_hex());
            }

            let mut buf = [0u8; REPORT_LEN];
            buf[1..].copy_from_slice(&frame.to_bytes());
            let written = self.device.write(&buf).context("HID write failed")?;
            if written != REPORT_LEN {
                bail!("HID short write: {written} of {REPORT_LEN} bytes");
            }

            self.drain_responses();
        }
        Ok(())
    }
}

/// Prints each frame as one hex line on stdout instead of sending it.
pub struct DryRun;

impl FrameSink for DryRun {
    fn send(&self, frames: &[HidFrame]) -> Result<()> {
        let mut out = std::io::stdout().lock();
        for frame in frames {
            if let Err(e) = writeln!(out, "{}", frame.to_hex()) {
                // The reader went away (e.g. piped into `head`); there is nothing left to do.
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    return Ok(());
                }
                return Err(e).context("Failed to write to stdout");
            }
        }
        Ok(())
    }
}

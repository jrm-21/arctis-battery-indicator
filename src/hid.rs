use std::io::Read;

use hidapi::{HidDevice, HidError};

pub struct Headphone {
    device: HidDevice,
    name: Option<String>,
    /// percentage in range [0,4]
    pub battery_state: u8,
    /// - 0: not connected
    /// - 1: charging
    /// - 3: discharging
    charging_state: u8,
}

impl Headphone {
    pub fn battery_percentage(&self) -> i32 {
        ((self.battery_state as f32 / 4.0) * 100.0) as i32
    }

    pub fn charging_status(&self) -> &str {
        match self.charging_state {
            1 => "Charging",
            3 => "Discharging",
            _ => "Disconnected",
        }
    }

    /// if return is Ok(true), state has changed
    pub fn update(&mut self) -> hidapi::HidResult<bool> {
        self.device.write(&[0x0, 0xb0])?;

        let mut buf = [0u8; 4];

        self.device.read_timeout(&mut buf, 100)?;

        // save old state
        let Headphone {
            battery_state: old_battery,
            charging_state: old_charging,
            ..
        } = *self;

        self.battery_state = buf[2];
        self.charging_state = buf[3];

        Ok(self.battery_state != old_battery || self.charging_state != old_charging)
    }
}

impl std::fmt::Display for Headphone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            f.write_fmt(format_args!("{}: ", name))?;
        }

        f.write_fmt(format_args!(
            "{}% - {}",
            self.battery_percentage(),
            self.charging_status(),
        ))?;

        Ok(())
    }
}

pub struct Controller {
    api: hidapi::HidApi,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            api: hidapi::HidApi::new().expect("Failed to initialize hidapi"),
        }
    }

    pub fn get_device(&self) -> Option<Headphone> {
        let devices = self.api.device_list().filter(|d| {
            // SteelSeries HID vendor ID
            // Arctis Nova 7 product ID
            // https://devicehunt.com/search/type/usb/vendor/1038/device/any
            d.vendor_id() == 0x1038 && d.product_id() == 0x2202
        });

        for device in devices {
            let device = match self.api.open_path(device.path()) {
                Ok(d) => d,
                Err(_) => continue,
            };

            // try to write to device
            if device.write(&[0x0, 0xb0]).is_err() {
                continue;
            }
            let mut buf = [0u8; 4];

            // timeout because some devices will block here indefinitely
            device.read_timeout(&mut buf, 100).unwrap();

            // On a successful read, the first byte will contain non-zero report number
            if buf[0] == 0 {
                continue;
            }

            let device_name = device.get_product_string().unwrap_or(None);
            return Some(Headphone {
                device,
                name: device_name,
                battery_state: 0,
                charging_state: 0,
            });
        }

        None
    }
}

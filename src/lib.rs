mod datatypes;
mod tests;

use hidapi;
use hidapi::{HidApi, HidDevice, HidResult};
use crate::datatypes::LightingUpdateMessage;

/// Returns the first HidDevice that supports the polling
/// 0x04 0x18 message and doesn't return an error
pub fn get_keeb_hid_device_by_id(pid: u16, vid: u16) -> Option<HidDevice> {
    match HidApi::new() {
        Ok(api) => {
            for device in api.device_list() {
                // println!("vendor: {:04x} '{}', product: {:04x} '{}', SN: {}",
                //          device.vendor_id(),
                //          device.manufacturer_string().unwrap_or("NIL"),
                //          device.product_id(),
                //          device.product_string().unwrap_or("NIL"),
                //          device.serial_number().unwrap_or("NIL"));

                if device.product_id() == pid && device.vendor_id() == vid {
                    match device.open_device(&api) {
                        Ok(d) => {
                            let data = [00, 0x04, 0x18];
                            match d.send_feature_report(&data) {
                                Ok(_) => return Some(d),
                                Err(e) => {
                                    eprintln!("Failed to poll HID device: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error opening hid device: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    };

    return None;
}

pub fn list_hid_devices() {
    match HidApi::new() {
        Ok(api) => {
            for device in api.device_list() {
                println!("vendor: {:04x} '{}', product: {:04x} '{}', SN: {}",
                         device.vendor_id(),
                         device.manufacturer_string().unwrap_or("NIL"),
                         device.product_id(),
                         device.product_string().unwrap_or("NIL"),
                         device.serial_number().unwrap_or("NIL"));
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

pub fn send_lighting_update_message(lum: &LightingUpdateMessage, device: &HidDevice) -> HidResult<()> {
    device.set_blocking_mode(true);
    let data_blocks = lum.construct_feature_report_data_blocks();

    for (block_num, block) in data_blocks.iter().enumerate() {
        device.send_feature_report(block)?;

        match block_num {
            0 | 1 | 3 | 4 | 23 | 25 => {
                let mut freport = [0; 65];
                device.get_feature_report(&mut freport).unwrap();
            }
            _ => {}
        }
    }

    Ok(())
}
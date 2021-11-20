#![cfg(test)]

use std::array::IntoIter;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::thread::sleep;
use std::time::Duration;
use crate::{get_keeb_hid_device_by_id, list_hid_devices, send_lighting_update_message};
use crate::datatypes::{Direction, LightingUpdateMessage, Mode, mode_preset, rgb};

const PRODUCT_ID: u16 = 0x24f;
const VENDOR_ID: u16 = 0x5ac;

#[test]
fn test_hid_send_feature_report() {
    let device = get_keeb_hid_device_by_id(PRODUCT_ID, VENDOR_ID).unwrap();
    let data = [
        00
    ];
    if let Err(e) = device.send_feature_report(&data) {
        match device.check_error() {
            Ok(err) =>
                eprintln!("Hidapi error found: {}", err),
            Err(e) =>
                eprintln!("Error sending feature report, but unable to detect what the error is: {}", e)
        }
    } else {
        println!("Success!");
    }
}

#[test]
fn test_send_lighting_update_message() {
    let lum_rolling =
        LightingUpdateMessage::set_active_mode(
            mode_preset(
                Mode::Rolling,
                rgb(0, 0, 0),
                true,
                2,
                6,
                Direction::Left,
            ));

    let lum_emerald = LightingUpdateMessage::set_active_mode(
        mode_preset(
            Mode::Static,
            rgb(25, 255, 50),
            false,
            16,
            1,
            Direction::Right
        ));

    let lum_off = LightingUpdateMessage::set_backlight_off();

    let lum_ripples = LightingUpdateMessage::set_active_mode(
        mode_preset(
            Mode::Ripples,
            rgb(25, 255, 50),
            true,
            16,
            13,
            Direction::Right
        ));

    let device = get_keeb_hid_device_by_id(PRODUCT_ID, VENDOR_ID).unwrap();

    println!("set to rolling left");
    send_lighting_update_message(&lum_rolling, &device).unwrap();
    sleep(Duration::from_secs_f64(2.5));
    println!("set to off");
    send_lighting_update_message(&lum_off, &device).unwrap();
    sleep(Duration::from_secs_f64(1.0));
    println!("set to emerald static");
    send_lighting_update_message(&lum_emerald, &device).unwrap();
    sleep(Duration::from_secs_f64(2.5));
    println!("set to full color ripples");
    send_lighting_update_message(&lum_ripples, &device).unwrap();
}

#[test]
fn test_if_typing_allow_during_message_update() {
    use crate::datatypes::{Key::*, rgb};

    let orange = rgb(255, 128, 15);
    let cyan = rgb(15, 240, 255);
    let lum1 = LightingUpdateMessage::set_user_defined(
        16,
        HashMap::from_iter(IntoIter::new([
            (Q, orange),
            (W, orange),
            (E, orange)
        ]))
    );
    let lum2 = LightingUpdateMessage::set_user_defined(
        16,
        HashMap::from_iter(IntoIter::new([
            (A, cyan),
            (S, cyan),
            (D, cyan)
        ]))
    );

    let device = get_keeb_hid_device_by_id(PRODUCT_ID, VENDOR_ID).unwrap();

    for _ in 0..3 {
        send_lighting_update_message(&lum1, &device);
        sleep(Duration::from_secs(1));
        send_lighting_update_message(&lum2, &device);
        sleep(Duration::from_secs(1));
    }
}

#[test]
fn test_send_lighting_update_message_verbose_manual() {
    let device = get_keeb_hid_device_by_id(PRODUCT_ID, VENDOR_ID).unwrap();
    device.set_blocking_mode(true);

    let lum =
        LightingUpdateMessage::set_active_mode(
            mode_preset(
                Mode::Rolling,
                rgb(0, 0, 0),
                true,
                2,
                6,
                Direction::Left,
            ));

    let data_blocks = lum.construct_feature_report_data_blocks();

    for (block_num, block) in data_blocks.iter().enumerate() {
        if let Err(e) = device.send_feature_report(block) {
            match device.check_error() {
                Ok(err) =>
                    eprintln!("Sending block {} Hidapi error: {}", block_num, err),
                Err(e) =>
                    println!("Err sending block {}, but unable to detect what the error is: {}", block_num, e)
            }
        } else {
            println!("Sending block {} success!", block_num);

            match block_num {
                0 | 1 | 3 | 4 | 23 | 25 => {
                    let mut freport = [0; 65];
                    device.get_feature_report(&mut freport).unwrap();
                    println!("Response from block {}: {:02X?}", block_num + 1, freport);
                }
                _ => {}
            }
        }
    }
}

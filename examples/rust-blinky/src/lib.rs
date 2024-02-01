// Copyright (C) 2024 Maik Lorenz
//
// This file is subject to the terms and conditions of the GNU Lesser
// General Public License v2.1. See the file LICENSE in the top level
// directory for more details.
#![no_std]

use riot_wrappers::ztimer;
use riot_wrappers::{
    gpio::{InputMode, OutputGPIO, OutputMode, GPIO},
    println, riot_main,
};
extern crate rust_riotmodules;

// LED 'User LD2': Arduino D13 -> PA5 (pin 21)
const LD2_PORT: u32 = 0;
const LD2_PIN: u32 = 5;

// Button 'B1 USER': PC13 (Pin 2)
const BTN_PORT: u32 = 2;
const BTN_PIN: u32 = 13;

riot_main!(main);

fn main() {
    println!("Hello Rust!");

    let mut led_ld2 = GPIO::from_port_and_pin(LD2_PORT, LD2_PIN)
        .expect("Error configuring 'LD2' pin")
        .configure_as_output(OutputMode::Out)
        .expect("Error configuring 'LD2' LED");
    led_ld2.set_low();

    let btn_user = GPIO::from_port_and_pin(BTN_PORT, BTN_PIN)
        .expect("Error configuring 'B1 USER' pin")
        .configure_as_input(InputMode::InPullDown)
        .expect("Error configuring 'B1 USER' button");

    let timer = ztimer::Clock::sec();
    timer.sleep_ticks(3);

    let mut led_state = false;
    loop {
        println!("");
        // Toggle LED
        led_state = !led_state;
        println!("Turn LD2 LED {}", if led_state { "on" } else { "off" });
        led_ld2.set_state(led_state);

        // Get user-button state
        println!(
            "Button is {}",
            if btn_user.is_high() {
                "released"
            } else {
                "pressed"
            }
        );
        timer.sleep_ticks(1);
    }
}

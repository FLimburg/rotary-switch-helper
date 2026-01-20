# Rotary Switch Helper
<a href="https://crates.io/crates/rotary-switch-helper"><img alt="crates.io" src="https://img.shields.io/crates/v/rotary-switch-helper"></a>
<a href="https://github.com/FLimburg/rotary-switch-helper/actions"><img alt="actions" src="https://github.com/FLimburg/rotary-switch-helper/actions/workflows/rust.yml/badge.svg"></a>

A Rust library for handling rotary encoders and switches on Raspberry Pi.

## Overview

This library provides a clean, thread-safe interface for working with rotary encoders and switches on Raspberry Pi. It handles the debouncing, state management, and event callbacks, allowing you to focus on your application logic rather than hardware details.

## Features

- Support for standalone rotary encoders
- Support for standalone switches with long press detection
- Support for rotary encoders with built-in switches (shifted mode)
- Thread-safe design using atomic operations
- Customizable callback functions for rotation and switch events
- Normal and "shifted" mode for rotary encoders with switches
- Comprehensive test suite with hardware mocking and hardware integration tests

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rotary-switch-helper = "0.2.0"
```

## Usage Examples

### Recommended: Using the PiInput Wrapper

The recommended way to use this library is through the `PiInput` wrapper, which manages all your encoders and handles GPIO initialization:

```rust
use rotary_switch_helper::{
    PiInput, 
    SwitchDefinition, 
    RotaryDefinition,
    rotary_encoder::Direction
};

// Callback for rotary encoders
fn handle_rotation(name: &str, direction: Direction) {
    match direction {
        Direction::Clockwise => println!("{} turned clockwise", name),
        Direction::CounterClockwise => println!("{} turned counter-clockwise", name),
        Direction::None => {}
    }
}

// Callback for switches
fn handle_switch(name: &str, pressed: bool) {
    if pressed {
        println!("{} pressed", name);
    } else {
        println!("{} released", name);
    }
}

fn main() -> anyhow::Result<()> {
    // Define switches (with optional long press detection)
    let switches = vec![
        SwitchDefinition {
            name: "button1".to_string(),
            name_long_press: None,  // No long press detection
            sw_pin: 22,
            time_threshold: None,
            callback: handle_switch,
        },
        SwitchDefinition {
            name: "button2".to_string(),
            name_long_press: Some("button2_long".to_string()),  // Enable long press
            sw_pin: 23,
            time_threshold: Some(std::time::Duration::from_secs(2)),  // 2 second threshold
            callback: handle_switch,
        },
    ];

    // Define rotary encoders
    let rotaries = vec![
        RotaryDefinition {
            name: "volume".to_string(),
            name_shifted: None,  // No shifted mode
            sw_pin: None,  // No built-in switch
            dt_pin: 17,
            clk_pin: 27,
            callback: handle_rotation,
        },
    ];

    // Define rotary encoders with built-in switches (shifted mode)
    let rotary_switches = vec![
        RotaryDefinition {
            name: "menu_selector".to_string(),
            name_shifted: Some("menu_selector_shifted".to_string()),
            dt_pin: 5,
            clk_pin: 6,
            sw_pin: Some(13),  // Built-in switch pin
            callback: handle_rotation,
        },
    ];

    // Create PiInput instance that manages all encoders
    // Combine all rotary encoders (with and without switches) into one vector
    let all_rotaries = [rotaries, rotary_switches].concat();
    let _input = PiInput::new(&switches, &all_rotaries)?;

    // Keep the program running
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

```

### Alternative: Direct Component Usage

While using the `PiInput` wrapper is recommended, you can also use the individual components directly if needed. Note that when using components directly, you'll need to manage the GPIO initialization yourself.

#### Basic Rotary Encoder

```rust
use rotary_switch_helper::rotary_encoder::{Encoder, Direction};
use rppal::gpio::Gpio;

fn handle_rotation(name: &str, direction: Direction) {
    match direction {
        Direction::Clockwise => println!("{} turned clockwise", name),
        Direction::CounterClockwise => println!("{} turned counter-clockwise", name),
        Direction::None => {}
    }
}

fn main() -> anyhow::Result<()> {
    let gpio = Gpio::new()?;

    // Initialize encoder with name, shifted name, GPIO interface, DT pin, CLK pin, switch pin, and callback
    let _encoder = Encoder::new(
        "volume",
        None,        // No shifted name
        &gpio,
        17,          // DT pin
        27,          // CLK pin
        None,        // No switch pin
        handle_rotation
    )?;

    // Keep the program running
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

#### Switch (with optional long press detection)

```rust
use rotary_switch_helper::switch_encoder;
use rppal::gpio::Gpio;
use std::time::Duration;

fn handle_switch(name: &str, pressed: bool) {
    if pressed {
        println!("{} pressed", name);
    } else {
        println!("{} released", name);
    }
}

fn main() -> anyhow::Result<()> {
    let gpio = Gpio::new()?;
    
    // Initialize switch with long press detection
    let _switch = switch_encoder::Encoder::new(
        "button",                           // Normal press name
        Some("button_long"),                // Long press name
        &gpio,
        22,                                 // Switch pin
        Some(Duration::from_secs(2)),       // 2 second threshold for long press
        handle_switch
    )?;
    
    // Keep the program running
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

#### Rotary Encoder with Built-in Switch (Shifted Mode)

```rust
use rotary_switch_helper::rotary_encoder;
use rotary_switch_helper::rotary_encoder::Direction;
use rppal::gpio::Gpio;

fn handle_rotation(name: &str, direction: Direction) {
    match direction {
        Direction::Clockwise => println!("{} turned clockwise", name),
        Direction::CounterClockwise => println!("{} turned counter-clockwise", name),
        Direction::None => {}
    }
}

fn main() -> anyhow::Result<()> {
    let gpio = Gpio::new()?;
    
    // Initialize rotary encoder with built-in switch for shifted mode
    let _encoder = rotary_encoder::Encoder::new(
        "encoder_with_switch",              // Normal name
        Some("encoder_with_switch_shifted"), // Shifted name (when switch is pressed)
        &gpio,
        17,              // DT pin
        27,              // CLK pin
        Some(22),        // Switch pin
        handle_rotation
    )?;
    
    // Keep the program running
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

## How It Works

### Rotary Encoder State Machine

The library implements a state machine to track the rotary encoder's state transitions. 
The encoder uses two pins (DT and CLK) which create a Gray code pattern during rotation:

```
State | CLK | DT
------|-----|----
  0   |  0  |  0   (Resting position)
  1   |  1  |  0   (First step clockwise)
  3   |  1  |  1   (Second step)
  2   |  0  |  1   (Third step)
  0   |  0  |  0   (Back to resting - complete clockwise turn)
```

For counter-clockwise rotation, the sequence is reversed.

The library detects these state transitions and calls the provided callback function when a full rotation is detected.

### Switch Handling

Switches are debounced and trigger callbacks on both press and release events. The library also supports long press detection - when configured with a time threshold and a long press name, the switch will trigger different callbacks for normal presses versus long presses (when the button is held down beyond the threshold).

### Shifted Mode

When using a rotary encoder with a built-in switch, the library supports a "shifted" mode. When the switch is pressed, the rotary encoder enters shifted mode, allowing you to implement different behaviors for the same physical control.

## Testing

The library includes a comprehensive test suite with both unit tests and hardware integration tests.

### Unit Tests

Run the unit tests (which use mock objects to simulate hardware interactions) with:

```bash
cargo test
```

### Hardware Integration Tests

The library also includes hardware integration tests that require actual Raspberry Pi hardware with encoders connected. These tests verify end-to-end functionality including GPIO interrupts and callbacks.

Run hardware tests (requires manual interaction with physical hardware):

```bash
# Run all hardware tests
cargo test --test hardware_integration_test -- --ignored --nocapture --test-threads=1

# Run a specific hardware test
cargo test --test hardware_integration_test test_rotary_clockwise -- --ignored --nocapture --test-threads=1
```

See `tests/hardware_integration_test.rs` for hardware setup instructions.

## Requirements

- Rust 1.56 or higher
- Raspberry Pi with GPIO pins
- `rppal` crate compatibility (most Raspberry Pi models)

## License

This project is licensed under the [MIT License](LICENSE).

## Discalimer

Unittests and Readme are written by AI.
You might want to prefer the actual code for a deeper/more correct understanding.

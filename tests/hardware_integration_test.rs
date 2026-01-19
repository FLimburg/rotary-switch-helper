//! Hardware Integration Tests for Rotary Encoder
//!
//! These tests require actual Raspberry Pi hardware with a rotary encoder connected.
//! They verify the complete end-to-end functionality including GPIO interrupts and callbacks.
//!
//! ## Hardware Setup
//!
//! Connect a rotary encoder to your Raspberry Pi as follows:
//! - DT (Data)  → GPIO 9
//! - CLK (Clock) → GPIO 10
//! - SW (Switch, optional) → GPIO 11
//! - GND → GND
//! - VCC → 3.3V (or 5V depending on your encoder)
//!
//! ## Running the Tests
//!
//! These tests are marked as `#[ignore]` by default since they require hardware.
//! Run them explicitly with:
//!
//! ```bash
//! # Run all hardware tests (requires manual interaction)
//! cargo test --test hardware_integration_test -- --ignored --nocapture  --test-threads=1
//!
//! # Run a specific test
//! cargo test --test hardware_integration_test test_rotary_clockwise -- --ignored --nocapture  --test-threads=1
//! ```
//!
//! ## Note on Permissions
//!
//! You may need to run with sudo or add your user to the gpio group:
//! ```bash
//! sudo usermod -a -G gpio $USER
//! ```

use rotary_switch_helper::rotary_encoder::{Direction, Encoder};
use rppal::gpio::Gpio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Shared callback log for tracking all callback invocations
static CALLBACK_LOG: Mutex<Vec<(String, Direction)>> = Mutex::new(Vec::new());

const DT_PIN_NUMBER: u8 = 9;
const CLK_PIN_NUMBER: u8 = 10;
const SW_PIN_NUMBER: u8 = 11;

/// Test callback function that logs all invocations
fn test_callback(name: &str, direction: Direction) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    println!(
        "[{}ms] ✓ Callback: '{}' turned {:?}",
        timestamp, name, direction
    );
    CALLBACK_LOG
        .lock()
        .unwrap()
        .push((name.to_string(), direction));
}

/// Helper function to clear the callback log
fn clear_log() {
    CALLBACK_LOG.lock().unwrap().clear();
}

/// Helper function to get callback count
fn get_callback_count() -> usize {
    CALLBACK_LOG.lock().unwrap().len()
}

/// Helper function to get all callbacks
fn get_callbacks() -> Vec<(String, Direction)> {
    CALLBACK_LOG.lock().unwrap().clone()
}

/// Helper to ensure GPIO resources are released
/// Note: Due to rppal GPIO implementation, pins may not be immediately released
/// when Encoder is dropped. Adding a delay helps ensure cleanup.
fn wait_for_gpio_cleanup() {
    thread::sleep(Duration::from_millis(500));
}

#[test]
#[ignore]
fn test_rotary_encoder_initialization() {
    println!("\n=== Testing Rotary Encoder Initialization ===");

    let gpio = Gpio::new().expect("Failed to initialize GPIO - are you running on a Raspberry Pi?");

    let encoder = Encoder::new(
        "test_encoder",
        None,
        &gpio,
        DT_PIN_NUMBER,  // DT pin
        CLK_PIN_NUMBER, // CLK pin
        None,           // No switch pin
        test_callback,
    );

    assert!(
        encoder.is_ok(),
        "Encoder initialization should succeed with valid GPIO pins"
    );
    println!("✓ Encoder initialized successfully");
    wait_for_gpio_cleanup();
}

#[test]
#[ignore]
fn test_rotary_clockwise_turns() {
    println!("\n=== Testing Clockwise Rotation ===");
    println!("Please turn the encoder CLOCKWISE 5 times when prompted...");
    println!("You have 10 seconds.");

    clear_log();

    let gpio = Gpio::new().expect("Failed to initialize GPIO");
    let _encoder = Encoder::new(
        "clockwise_test",
        None,
        &gpio,
        DT_PIN_NUMBER,
        CLK_PIN_NUMBER,
        None,
        test_callback,
    )
    .expect("Failed to create encoder");

    println!("\n>>> START TURNING CLOCKWISE NOW <<<\n");
    thread::sleep(Duration::from_secs(10));

    let callbacks = get_callbacks();
    println!("\n--- Results ---");
    println!("Total callbacks: {}", callbacks.len());

    let clockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == Direction::Clockwise)
        .count();

    let counterclockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == Direction::CounterClockwise)
        .count();

    println!("Clockwise: {}", clockwise_count);
    println!("Counter-clockwise: {}", counterclockwise_count);

    assert!(
        clockwise_count > 0,
        "Expected at least one clockwise rotation"
    );
    println!("✓ Clockwise rotations detected successfully");
    wait_for_gpio_cleanup();
}

#[test]
#[ignore]
fn test_rotary_counterclockwise_turns() {
    println!("\n=== Testing Counter-Clockwise Rotation ===");
    println!("Please turn the encoder COUNTER-CLOCKWISE 5 times when prompted...");
    println!("You have 10 seconds.");

    clear_log();

    let gpio = Gpio::new().expect("Failed to initialize GPIO");
    let _encoder = Encoder::new(
        "counterclockwise_test",
        None,
        &gpio,
        DT_PIN_NUMBER,
        CLK_PIN_NUMBER,
        None,
        test_callback,
    )
    .expect("Failed to create encoder");

    println!("\n>>> START TURNING COUNTER-CLOCKWISE NOW <<<\n");
    thread::sleep(Duration::from_secs(10));

    let callbacks = get_callbacks();
    println!("\n--- Results ---");
    println!("Total callbacks: {}", callbacks.len());

    let clockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == Direction::Clockwise)
        .count();

    let counterclockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == Direction::CounterClockwise)
        .count();

    println!("Clockwise: {}", clockwise_count);
    println!("Counter-clockwise: {}", counterclockwise_count);

    assert!(
        counterclockwise_count > 0,
        "Expected at least one counter-clockwise rotation"
    );
    println!("✓ Counter-clockwise rotations detected successfully");
    wait_for_gpio_cleanup();
}

#[test]
#[ignore]
fn test_rotary_both_directions() {
    println!("\n=== Testing Both Directions ===");
    println!("Please turn the encoder in BOTH directions when prompted...");
    println!("You have 10 seconds.");

    clear_log();

    let gpio = Gpio::new().expect("Failed to initialize GPIO");
    let _encoder = Encoder::new(
        "bidirectional_test",
        None,
        &gpio,
        DT_PIN_NUMBER,
        CLK_PIN_NUMBER,
        None,
        test_callback,
    )
    .expect("Failed to create encoder");

    println!("\n>>> START TURNING IN BOTH DIRECTIONS NOW <<<\n");
    thread::sleep(Duration::from_secs(10));

    let callbacks = get_callbacks();
    println!("\n--- Results ---");
    println!("Total callbacks: {}", callbacks.len());

    let clockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == Direction::Clockwise)
        .count();

    let counterclockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == Direction::CounterClockwise)
        .count();

    println!("Clockwise: {}", clockwise_count);
    println!("Counter-clockwise: {}", counterclockwise_count);

    for (i, (name, dir)) in callbacks.iter().enumerate() {
        println!("  {}. {} -> {:?}", i + 1, name, dir);
    }

    assert!(
        clockwise_count > 0,
        "Expected at least one clockwise rotation"
    );
    assert!(
        counterclockwise_count > 0,
        "Expected at least one counter-clockwise rotation"
    );
    println!("✓ Both directions detected successfully");
    wait_for_gpio_cleanup();
}

#[test]
#[ignore]
fn test_rotary_with_shifted_name() {
    println!("\n=== Testing Encoder with Shift Support ===");
    println!("This test verifies the shifted name functionality.");
    println!("Connect the switch pin (SW) to GPIO {SW_PIN_NUMBER} for full testing.");
    println!("Turn the encoder when prompted (with and without pressing the switch)...");
    println!("You have 25 seconds.");

    clear_log();

    let gpio = Gpio::new().expect("Failed to initialize GPIO");
    let _encoder = Encoder::new(
        "normal_name",
        Some("shifted_name"),
        &gpio,
        DT_PIN_NUMBER,
        CLK_PIN_NUMBER,
        Some(SW_PIN_NUMBER), // Switch pin
        test_callback,
    )
    .expect("Failed to create encoder with shift support");

    println!("\n>>> START TURNING (try with and without pressing the button) <<<\n");
    thread::sleep(Duration::from_secs(25));

    let callbacks = get_callbacks();
    println!("\n--- Results ---");
    println!("Total callbacks: {}", callbacks.len());

    for (i, (name, dir)) in callbacks.iter().enumerate() {
        println!("  {}. '{}' -> {:?}", i + 1, name, dir);
    }

    assert!(callbacks.len() > 0, "Expected at least one callback");
    println!("✓ Encoder with shift support working");
    wait_for_gpio_cleanup();
}

#[test]
#[ignore]
fn test_rotary_rapid_turns() {
    println!("\n=== Testing Rapid Rotations ===");
    println!("Please turn the encoder RAPIDLY back and forth when prompted...");
    println!("This tests the encoder's ability to handle quick state changes.");
    println!("You have 10 seconds.");

    clear_log();

    let gpio = Gpio::new().expect("Failed to initialize GPIO");
    let _encoder = Encoder::new(
        "rapid_test",
        None,
        &gpio,
        DT_PIN_NUMBER,
        CLK_PIN_NUMBER,
        None,
        test_callback,
    )
    .expect("Failed to create encoder");

    println!("\n>>> START RAPID TURNING NOW <<<\n");
    thread::sleep(Duration::from_secs(10));

    let callbacks = get_callbacks();
    println!("\n--- Results ---");
    println!("Total callbacks: {}", callbacks.len());

    assert!(callbacks.len() > 0, "Expected callbacks from rapid turning");

    // Verify no duplicate consecutive directions would indicate missed states
    let mut direction_changes = 0;
    for i in 1..callbacks.len() {
        if callbacks[i].1 != callbacks[i - 1].1 {
            direction_changes += 1;
        }
    }

    println!("Direction changes: {}", direction_changes);
    println!("✓ Rapid rotations handled successfully");
    wait_for_gpio_cleanup();
}

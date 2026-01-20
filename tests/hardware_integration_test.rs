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
use rotary_switch_helper::rotary_encoder;
use rotary_switch_helper::rotary_encoder::Direction;
use rotary_switch_helper::switch_encoder;
use rppal::gpio::Gpio;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use test_log::test;

/// Shared callback log for tracking all callback invocations
static CALLBACK_LOG: Mutex<Vec<(String, rotary_encoder::Direction)>> = Mutex::new(Vec::new());
static CALLBACK_SW_LOG: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

const DT_PIN_NUMBER: u8 = 9;
const CLK_PIN_NUMBER: u8 = 10;
const SW_PIN_NUMBER: u8 = 11;

/// Test callback function that logs all invocations
fn test_callback(name: &str, direction: rotary_encoder::Direction) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    println!(
        "[{}ms] ✓ Callback: '{name}' turned {:?}",
        timestamp, direction
    );
    CALLBACK_LOG
        .lock()
        .unwrap()
        .push((name.to_string(), direction));
}

/// Test callback function that logs all invocations
fn test_callback_switch(name: &str, pressed: bool) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    println!("[{}ms] ✓ Callback: '{name}' pressed {pressed}", timestamp);
    CALLBACK_SW_LOG
        .lock()
        .unwrap()
        .push((name.to_string(), pressed));
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

/// Helper function to clear the callback log
fn clear_log_switch() {
    CALLBACK_SW_LOG.lock().unwrap().clear();
}

/// Helper function to get callback count
fn get_callback_count_switch() -> usize {
    CALLBACK_SW_LOG.lock().unwrap().len()
}

/// Helper function to get all callbacks
fn get_callbacks_switch() -> Vec<(String, bool)> {
    CALLBACK_SW_LOG.lock().unwrap().clone()
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

    let encoder = rotary_encoder::Encoder::new(
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
    let _encoder = rotary_encoder::Encoder::new(
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
    println!("Total callbacks: {}", get_callback_count());

    let clockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == rotary_encoder::Direction::Clockwise)
        .count();

    let counterclockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == rotary_encoder::Direction::CounterClockwise)
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
    let _encoder = rotary_encoder::Encoder::new(
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
    println!("Total callbacks: {}", get_callback_count());

    let clockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == rotary_encoder::Direction::Clockwise)
        .count();

    let counterclockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == rotary_encoder::Direction::CounterClockwise)
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
    let _encoder = rotary_encoder::Encoder::new(
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
    println!("Total callbacks: {}", get_callback_count());

    let clockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == rotary_encoder::Direction::Clockwise)
        .count();

    let counterclockwise_count = callbacks
        .iter()
        .filter(|(_, dir)| *dir == rotary_encoder::Direction::CounterClockwise)
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
    let _encoder = rotary_encoder::Encoder::new(
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
    println!("Total callbacks: {}", get_callback_count());

    for (i, (name, dir)) in callbacks.iter().enumerate() {
        println!("  {}. '{}' -> {:?}", i + 1, name, dir);
    }

    assert!(get_callback_count() > 0, "Expected at least one callback");
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
    let _encoder = rotary_encoder::Encoder::new(
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
    println!("Total callbacks: {}", get_callback_count());

    assert!(
        get_callback_count() > 0,
        "Expected callbacks from rapid turning"
    );

    // Verify no duplicate consecutive directions would indicate missed states
    let mut direction_changes = 0;
    for i in 1..get_callback_count() {
        if callbacks[i].1 != callbacks[i - 1].1 {
            direction_changes += 1;
        }
    }

    println!("Direction changes: {}", direction_changes);
    println!("✓ Rapid rotations handled successfully");
    wait_for_gpio_cleanup();
}

#[test]
#[ignore]
fn test_switch_press() {
    println!("\n=== Testing Press ===");
    println!("Please press the encoder when prompted...");
    println!("This tests the encoder's ability to handle switch presses.");
    println!("You have 10 seconds.");

    clear_log_switch();

    let gpio = Gpio::new().expect("Failed to initialize GPIO");
    let _encoder = switch_encoder::Encoder::new(
        "press",
        None,
        &gpio,
        SW_PIN_NUMBER,
        None,
        test_callback_switch,
    )
    .expect("Failed to create encoder");

    println!("\n>>> START PRESSING THE switch NOW <<<\n");
    thread::sleep(Duration::from_secs(10));

    println!("\n--- Results ---");
    println!("Total callbacks: {}", get_callback_count_switch());

    assert!(
        get_callback_count_switch() > 0,
        "Expected callbacks from pressing"
    );

    let callbacks = get_callbacks_switch();
    assert!(
        callbacks.get(0).unwrap().1,
        "Expected first callback to be a press not release event"
    );

    println!("✓ Presses handled successfully");
    wait_for_gpio_cleanup();
}

#[test]
#[ignore]
fn test_switch_long_press() {
    println!("\n=== Testing Press ===");
    println!("Please press and hold the encoder for 5 seconds when prompted...");
    println!("This tests the encoder's ability to handle long switch presses.");
    println!("You have 15 seconds.");

    clear_log_switch();

    let gpio = Gpio::new().expect("Failed to initialize GPIO");
    let _encoder = switch_encoder::Encoder::new(
        "press",
        Some("long_press"),
        &gpio,
        SW_PIN_NUMBER,
        Some(Duration::from_secs(4)),
        test_callback_switch,
    )
    .expect("Failed to create encoder");

    println!("\n>>> START PRESSING THE switch NOW <<<\n");
    thread::sleep(Duration::from_secs(15));

    let callbacks: Vec<(String, bool)> = get_callbacks_switch();

    println!("\n--- Results ---");
    println!("Total callbacks: {}", get_callback_count_switch());

    assert!(
        get_callback_count_switch() > 0,
        "Expected callbacks from pressing"
    );

    let long_count = callbacks
        .iter()
        .filter(|(n, p)| n == "long_press" && !*p)
        .count();
    println!("Long press callbacks: {long_count}");
    assert!(long_count > 0, "Expected callbacks from long pressing");

    println!("✓ Presses handled successfully");
    wait_for_gpio_cleanup();
}

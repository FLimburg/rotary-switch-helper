use rppal::gpio::{Event, Gpio, InputPin, Level, Trigger};

use anyhow::{Result, anyhow};
use atomic_enum::atomic_enum;
use log::{error, trace};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// Direction of rotation
#[atomic_enum]
#[derive(PartialEq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub enum Pin {
    Dt,
    Clk,
}

#[derive(Debug)]
pub struct Encoder {
    name: Arc<String>,
    name_shifted: Arc<Option<String>>,
    dt_pin: InputPin,
    clk_pin: InputPin,
    sw_pin: Arc<Option<InputPin>>,
    state: Arc<AtomicU8>,
    direction: Arc<AtomicDirection>,
    callback: Arc<fn(&str, Direction)>,
}

impl Encoder {
    /// Create a new rotary encoder
    /// # Arguments
    /// * `encoder_name` - Name of the encoder
    /// * `encoder_name_shifted` - Name of the encoder when pressed
    /// * `gpio` - Gpio instance to use for the encoder
    /// * `dt_pin` - GPIO pin number for data (DT) encoder signal
    /// * `clk_pin` - GPIO pin number for clock (CLK) encoder signal
    /// * `callback` - Function to call when the encoder is turned
    pub fn new(
        encoder_name: &str,
        encoder_name_shifted: Option<&str>,
        gpio: &Gpio,
        dt_pin: u8,
        clk_pin: u8,
        sw_pin: Option<u8>,
        callback: fn(&str, Direction),
    ) -> Result<Self> {
        trace!(
            "Initializing GPIO for rotary encoder {}/{:?}",
            encoder_name, encoder_name_shifted
        );

        let dt = gpio.get(dt_pin)?.into_input_pullup();
        let clk = gpio.get(clk_pin)?.into_input_pullup();
        let sw = match sw_pin {
            None => None,
            Some(p) => Some(gpio.get(p)?.into_input_pullup()),
        };

        let mut encoder = Self {
            name: Arc::new(encoder_name.to_owned()),
            name_shifted: Arc::new(encoder_name_shifted.map(|s| s.to_owned())),
            dt_pin: dt,
            clk_pin: clk,
            sw_pin: Arc::new(sw),
            state: Arc::new(AtomicU8::new(0)),
            direction: Arc::new(AtomicDirection::new(Direction::None)),
            callback: Arc::new(callback),
        };

        encoder
            .enable_callbacks()
            .map_err(|e| anyhow!("Failed to enable callbacks: {}", e))?;
        trace!(
            "Rotary encoder {}/{:?} initialized",
            encoder.name, encoder_name_shifted
        );
        Ok(encoder)
    }

    fn update_state(
        old_state: u8,
        old_direction: Direction,
        pin: Pin,
        level: u8,
    ) -> Result<(u8, Direction, bool)> {
        let mut trigger = false;
        let new_state = match pin {
            Pin::Clk => (old_state & 0b10) + level,
            Pin::Dt => (old_state & 0b01) + (level << 1),
        };
        let trans_state = (old_state << 2) + new_state;

        let direction = match trans_state {
            0b0001 => Direction::Clockwise, // Resting position & Turned right 1
            0b0010 => Direction::CounterClockwise, // Resting position & Turned left 1
            0b0111 => Direction::Clockwise, // R1 or L3 position & Turned right 1
            0b0100 if old_direction == Direction::CounterClockwise => {
                // R1 or L3 position & Turned left  1
                trigger = true;
                Direction::CounterClockwise
            }
            0b1011 => Direction::CounterClockwise, // R3 or L1 position & Turned left 1
            0b1000 if old_direction == Direction::Clockwise => {
                // R3 or L1 position & Turned right 1
                trigger = true;
                Direction::Clockwise
            }
            0b1101 => Direction::CounterClockwise, // R2 or L2 position & Turned left 1
            0b1110 => Direction::Clockwise,        // R2 or L2 position & Turned right 1
            // this should not be possible with single pin transitions
            // 0b1100 if old_direction != Direction::None => {
            //     // R2 or L2 & Skipped an intermediate 01 or 10 state
            //     trigger = true;
            //     old_direction
            // }
            _ => Err(anyhow!(
                "Invalid state transition: from {:04b} / {:?} -> {:04b}",
                old_state,
                old_direction,
                trans_state
            ))?,
        };
        Ok((new_state, direction, trigger))
    }

    fn enable_callbacks(&mut self) -> Result<()> {
        trace!(
            "Enabling callbacks for rotary encoder {}/{:?}",
            self.name, self.name_shifted
        );

        let state = HashMap::from([
            (Pin::Dt, Arc::clone(&self.state)),
            (Pin::Clk, Arc::clone(&self.state)),
        ]);
        let callback = HashMap::from([
            (Pin::Dt, Arc::clone(&self.callback)),
            (Pin::Clk, Arc::clone(&self.callback)),
        ]);
        let direction = HashMap::from([
            (Pin::Dt, Arc::clone(&self.direction)),
            (Pin::Clk, Arc::clone(&self.direction)),
        ]);
        let name = HashMap::from([
            (Pin::Dt, Arc::clone(&self.name)),
            (Pin::Clk, Arc::clone(&self.name)),
        ]);
        let name_shifted = HashMap::from([
            (Pin::Dt, Arc::clone(&self.name_shifted)),
            (Pin::Clk, Arc::clone(&self.name_shifted)),
        ]);
        let sw_pin = HashMap::from([
            (Pin::Dt, Arc::clone(&self.sw_pin)),
            (Pin::Clk, Arc::clone(&self.sw_pin)),
        ]);

        let interrupt_handler = Arc::new(move |event_trigger: Trigger, pin: Pin| {
            let old_state = state[&pin].load(Ordering::SeqCst);
            let old_direction = direction[&pin].load(Ordering::SeqCst);
            if let Ok((new_state, new_direction, trigger)) = Encoder::update_state(
                old_state,
                old_direction,
                pin,
                match event_trigger {
                    Trigger::RisingEdge => 0,
                    Trigger::FallingEdge => 1,
                    _ => {
                        error!("Unexpected event trigger: {:?}", event_trigger);
                        return;
                    }
                } as u8,
            ) {
                state[&pin].store(new_state, Ordering::SeqCst);
                direction[&pin].store(new_direction, Ordering::SeqCst);
                if trigger {
                    match (name_shifted[&pin].as_ref(), sw_pin[&pin].as_ref()) {
                        (None, None) => {
                            trace!(
                                "Rotary encoder {} turned {:?}, triggering callback (shift not sonfigured)",
                                name[&pin], new_direction
                            );
                            callback[&pin](&name[&pin], new_direction);
                        }
                        (Some(name_shift), Some(sp)) => match sp.read() == Level::High {
                            false => {
                                trace!(
                                    "Rotary encoder {:?} turned {:?}, triggering shifted callback",
                                    name_shift, new_direction
                                );
                                callback[&pin](name_shift, new_direction);
                            }
                            true => {
                                trace!(
                                    "Rotary encoder {} turned {:?}, triggering callback",
                                    name[&pin], new_direction
                                );
                                callback[&pin](&name[&pin], new_direction);
                            }
                        },
                        (_, _) => {
                            error!(
                                "Both sw_pin (is {:?}) and name shifted (is {:?}) must be defined!",
                                *name_shifted[&pin], *sw_pin[&pin]
                            )
                        }
                    }
                }
            }
        });
        let handler_clone = Arc::clone(&interrupt_handler);

        self.dt_pin
            .set_async_interrupt(Trigger::Both, None, move |event: Event| {
                handler_clone(event.trigger, Pin::Dt);
            })?;

        self.clk_pin
            .set_async_interrupt(Trigger::Both, None, move |event: Event| {
                interrupt_handler(event.trigger, Pin::Clk);
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_state_from_rest_clockwise() {
        // From resting state (00), CLK goes high -> transition 0001
        let result = Encoder::update_state(0b00, Direction::None, Pin::Clk, 1);
        assert!(result.is_ok());
        let (new_state, direction, trigger) = result.unwrap();
        assert_eq!(new_state, 0b01);
        assert_eq!(direction, Direction::Clockwise);
        assert_eq!(trigger, false);
    }

    #[test]
    fn test_update_state_from_rest_counterclockwise() {
        // From resting state (00), DT goes high -> transition 0010
        let result = Encoder::update_state(0b00, Direction::None, Pin::Dt, 1);
        assert!(result.is_ok());
        let (new_state, direction, trigger) = result.unwrap();
        assert_eq!(new_state, 0b10);
        assert_eq!(direction, Direction::CounterClockwise);
        assert_eq!(trigger, false);
    }

    #[test]
    fn test_update_state_clockwise_complete_rotation() {
        // Simulate a complete clockwise rotation sequence: 00 -> 01 -> 11 -> 10 -> 00

        // Step 1: 00 -> 01 (CLK high)
        let (state, direction, trigger) =
            Encoder::update_state(0b00, Direction::None, Pin::Clk, 1).unwrap();
        assert_eq!(state, 0b01);
        assert_eq!(direction, Direction::Clockwise);
        assert!(!trigger);

        // Step 2: 01 -> 11 (DT high) - transition 0111
        let (state, direction, trigger) =
            Encoder::update_state(0b01, Direction::Clockwise, Pin::Dt, 1).unwrap();
        assert_eq!(state, 0b11);
        assert_eq!(direction, Direction::Clockwise);
        assert!(!trigger);

        // Step 3: 11 -> 10 (CLK low) - transition 1110
        let (state, direction, trigger) =
            Encoder::update_state(0b11, Direction::Clockwise, Pin::Clk, 0).unwrap();
        assert_eq!(state, 0b10);
        assert_eq!(direction, Direction::Clockwise);
        assert!(!trigger);

        // Step 4: 10 -> 00 (DT low) - transition 1000, should trigger
        let (state, direction, trigger) =
            Encoder::update_state(0b10, Direction::Clockwise, Pin::Dt, 0).unwrap();
        assert_eq!(state, 0b00);
        assert_eq!(direction, Direction::Clockwise);
        assert!(trigger, "Should trigger callback on complete rotation");
    }

    #[test]
    fn test_update_state_counterclockwise_complete_rotation() {
        // Simulate a complete counter-clockwise rotation: 00 -> 10 -> 11 -> 01 -> 00

        // Step 1: 00 -> 10 (DT high)
        let (state, direction, trigger) =
            Encoder::update_state(0b00, Direction::None, Pin::Dt, 1).unwrap();
        assert_eq!(state, 0b10);
        assert_eq!(direction, Direction::CounterClockwise);
        assert!(!trigger);

        // Step 2: 10 -> 11 (CLK high) - transition 1011
        let (state, direction, trigger) =
            Encoder::update_state(0b10, Direction::CounterClockwise, Pin::Clk, 1).unwrap();
        assert_eq!(state, 0b11);
        assert_eq!(direction, Direction::CounterClockwise);
        assert!(!trigger);

        // Step 3: 11 -> 01 (DT low) - transition 1101
        let (state, direction, trigger) =
            Encoder::update_state(0b11, Direction::CounterClockwise, Pin::Dt, 0).unwrap();
        assert_eq!(state, 0b01);
        assert_eq!(direction, Direction::CounterClockwise);
        assert!(!trigger);

        // Step 4: 01 -> 00 (CLK low) - transition 0100, should trigger
        let (state, direction, trigger) =
            Encoder::update_state(0b01, Direction::CounterClockwise, Pin::Clk, 0).unwrap();
        assert_eq!(state, 0b00);
        assert_eq!(direction, Direction::CounterClockwise);
        assert!(trigger, "Should trigger callback on complete rotation");
    }

    #[test]
    fn test_update_state_transition_0111() {
        // Transition 0111: from state 01, DT goes high
        let (new_state, direction, trigger) =
            Encoder::update_state(0b01, Direction::Clockwise, Pin::Dt, 1).unwrap();
        assert_eq!(new_state, 0b11);
        assert_eq!(direction, Direction::Clockwise);
        assert!(!trigger);
    }

    #[test]
    fn test_update_state_transition_0100_trigger() {
        // Transition 0100 with CCW direction should trigger
        let (new_state, direction, trigger) =
            Encoder::update_state(0b01, Direction::CounterClockwise, Pin::Clk, 0).unwrap();
        assert_eq!(new_state, 0b00);
        assert_eq!(direction, Direction::CounterClockwise);
        assert!(trigger);
    }

    #[test]
    fn test_update_state_transition_1011() {
        // Transition 1011: from state 10, CLK goes high
        let (new_state, direction, trigger) =
            Encoder::update_state(0b10, Direction::CounterClockwise, Pin::Clk, 1).unwrap();
        assert_eq!(new_state, 0b11);
        assert_eq!(direction, Direction::CounterClockwise);
        assert!(!trigger);
    }

    #[test]
    fn test_update_state_transition_1000_trigger() {
        // Transition 1000 with CW direction should trigger
        let (new_state, direction, trigger) =
            Encoder::update_state(0b10, Direction::Clockwise, Pin::Dt, 0).unwrap();
        assert_eq!(new_state, 0b00);
        assert_eq!(direction, Direction::Clockwise);
        assert!(trigger);
    }

    #[test]
    fn test_update_state_transition_1101() {
        // Transition 1101: from state 11, DT goes low
        let (new_state, direction, trigger) =
            Encoder::update_state(0b11, Direction::CounterClockwise, Pin::Dt, 0).unwrap();
        assert_eq!(new_state, 0b01);
        assert_eq!(direction, Direction::CounterClockwise);
        assert!(!trigger);
    }

    #[test]
    fn test_update_state_transition_1110() {
        // Transition 1110: from state 11, CLK goes low
        let (new_state, direction, trigger) =
            Encoder::update_state(0b11, Direction::Clockwise, Pin::Clk, 0).unwrap();
        assert_eq!(new_state, 0b10);
        assert_eq!(direction, Direction::Clockwise);
        assert!(!trigger);
    }

    #[test]
    fn test_update_state_invalid_transition() {
        // Test an invalid state transition (e.g., 0000)
        let result = Encoder::update_state(0b00, Direction::None, Pin::Clk, 0);
        assert!(result.is_err(), "Transition 0000 should be invalid");
    }

    #[test]
    fn test_update_state_pin_dt_updates_correct_bits() {
        // DT pin should update bit 1 (second bit)
        let (new_state, _, _) = Encoder::update_state(0b00, Direction::None, Pin::Dt, 1).unwrap();
        assert_eq!(new_state, 0b10, "DT=1 should set bit 1");

        let (new_state, _, _) =
            Encoder::update_state(0b11, Direction::Clockwise, Pin::Dt, 0).unwrap();
        assert_eq!(new_state, 0b01, "DT=0 should clear bit 1");
    }

    #[test]
    fn test_update_state_pin_clk_updates_correct_bits() {
        // CLK pin should update bit 0 (first bit)
        let (new_state, _, _) = Encoder::update_state(0b00, Direction::None, Pin::Clk, 1).unwrap();
        assert_eq!(new_state, 0b01, "CLK=1 should set bit 0");

        let (new_state, _, _) =
            Encoder::update_state(0b11, Direction::Clockwise, Pin::Clk, 0).unwrap();
        assert_eq!(new_state, 0b10, "CLK=0 should clear bit 0");
    }
}

use rppal::gpio::{Event, Gpio, InputPin, Trigger};

use anyhow::{Result, anyhow};
use atomic_enum::atomic_enum;
use log::{error, trace};
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pin {
    Dt,
    Clk,
}

#[derive(Debug)]
pub struct Encoder {
    name: Arc<String>,
    dt_pin: InputPin,
    clk_pin: InputPin,
    state: Arc<AtomicU8>,
    direction: Arc<AtomicDirection>,
    callback: Arc<fn(&str, Direction)>,
}

impl Encoder {
    /// Create a new rotary encoder
    /// # Arguments
    /// * `name` - Name of the encoder
    /// * `gpio` - Gpio instance to use for the encoder
    /// * `dt_pin` - GPIO pin number for data (DT) encoder signal
    /// * `clk_pin` - GPIO pin number for clock (CLK) encoder signal
    /// * `callback` - Function to call when the encoder is turned
    pub fn new(
        encoder_name: &str,
        gpio: &Gpio,
        dt_pin: u8,
        clk_pin: u8,
        callback: fn(&str, Direction),
    ) -> Result<Self> {
        trace!("Initializing GPIO for rotary encoder {}", encoder_name);
        let name = encoder_name.to_owned();

        let dt = gpio.get(dt_pin)?.into_input_pullup();
        let clk = gpio.get(clk_pin)?.into_input_pullup();

        let mut encoder = Self {
            name: Arc::new(name),
            dt_pin: dt,
            clk_pin: clk,
            state: Arc::new(AtomicU8::new(0)),
            direction: Arc::new(AtomicDirection::new(Direction::None)),
            callback: Arc::new(callback),
        };

        encoder
            .enable_callbacks()
            .map_err(|e| anyhow!("Failed to enable callbacks: {}", e))?;
        trace!("Rotary encoder {} initialized", encoder.name);
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
            0b1100 if old_direction != Direction::None => {
                // R2 or L2 & Skipped an intermediate 01 or 10 state
                trigger = true;
                old_direction
            }
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
        trace!("Enabling callbacks for rotary encoder {}", self.name);
        let mut state = Arc::clone(&self.state);
        let mut callback = Arc::clone(&self.callback);
        let mut direction = Arc::clone(&self.direction);
        let mut name = Arc::clone(&self.name);
        self.dt_pin
            .set_async_interrupt(Trigger::Both, None, move |event: Event| {
                let old_state = state.load(Ordering::SeqCst);
                let old_direction = direction.load(Ordering::SeqCst);
                if let Ok((new_state, new_direction, trigger)) = Encoder::update_state(
                    old_state,
                    old_direction,
                    Pin::Dt,
                    match event.trigger {
                        Trigger::RisingEdge => 0,
                        Trigger::FallingEdge => 1,
                        _ => {
                            error!("Unexpected event trigger: {:?}", event.trigger);
                            return;
                        }
                    } as u8,
                ) {
                    state.store(new_state, Ordering::SeqCst);
                    direction.store(new_direction, Ordering::SeqCst);
                    if trigger {
                        trace!(
                            "Rotary encoder {} turned {:?}, triggering callback",
                            name, new_direction
                        );
                        callback(&name, new_direction);
                    }
                }
            })?;

        state = Arc::clone(&self.state);
        callback = Arc::clone(&self.callback);
        direction = Arc::clone(&self.direction);
        name = Arc::clone(&self.name);
        self.clk_pin
            .set_async_interrupt(Trigger::Both, None, move |event: Event| {
                let old_state = state.load(Ordering::SeqCst);
                let old_direction = direction.load(Ordering::SeqCst);
                if let Ok((new_state, new_direction, trigger)) = Encoder::update_state(
                    old_state,
                    old_direction,
                    Pin::Clk,
                    match event.trigger {
                        Trigger::RisingEdge => 0,
                        Trigger::FallingEdge => 1,
                        _ => {
                            error!("Unexpected event trigger: {:?}", event.trigger);
                            return;
                        }
                    } as u8,
                ) {
                    state.store(new_state, Ordering::SeqCst);
                    direction.store(new_direction, Ordering::SeqCst);
                    if trigger {
                        trace!(
                            "Rotary encoder {} turned {:?}, triggering callback",
                            name, new_direction
                        );
                        callback(&name, new_direction);
                    }
                }
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::time::Duration;

    // Mock structures for testing without real GPIO hardware
    #[allow(dead_code)]
    struct MockGpio {}

    struct MockInputPin {
        callback: Option<Box<dyn FnMut(Event) + Send>>,
    }

    #[allow(dead_code)]
    impl MockGpio {
        #[allow(dead_code)]
        fn new() -> Self {
            MockGpio {}
        }

        #[allow(dead_code)]
        fn get(&self, _pin: u8) -> Result<MockPin> {
            Ok(MockPin {})
        }
    }

    #[allow(dead_code)]
    struct MockPin {}

    #[allow(dead_code)]
    impl MockPin {
        #[allow(dead_code)]
        fn into_input_pullup(self) -> MockInputPin {
            MockInputPin { callback: None }
        }
    }

    impl MockInputPin {
        fn set_async_interrupt<F>(
            &mut self,
            _trigger: Trigger,
            _timeout: Option<Duration>,
            callback: F,
        ) -> Result<()>
        where
            F: FnMut(Event) + Send + 'static,
        {
            self.callback = Some(Box::new(callback));
            Ok(())
        }
        
        fn simulate_event(&mut self, event: Event) {
            if let Some(callback) = &mut self.callback {
                callback(event);
            }
        }
    }

    // This wrapper allows us to test the Encoder without real GPIO
    struct TestEncoder {
        name: String,
        dt_pin: Arc<Mutex<MockInputPin>>,
        clk_pin: Arc<Mutex<MockInputPin>>,
        state: Arc<AtomicU8>,
        direction: Arc<AtomicDirection>,
    }

    impl TestEncoder {
        fn new(name: &str) -> Self {
            TestEncoder {
                name: name.to_owned(),
                dt_pin: Arc::new(Mutex::new(MockInputPin { callback: None })),
                clk_pin: Arc::new(Mutex::new(MockInputPin { callback: None })),
                state: Arc::new(AtomicU8::new(0)),
                direction: Arc::new(AtomicDirection::new(Direction::None)),
            }
        }

        fn setup(&self, callback: fn(&str, Direction)) -> Result<()> {
            let name = Arc::new(self.name.clone());
            let state = Arc::clone(&self.state);
            let direction = Arc::clone(&self.direction);
            let name_clone = Arc::clone(&name);
            let state_clone = Arc::clone(&state);
            let direction_clone = Arc::clone(&direction);

            // DT pin callback setup
            let mut dt_pin = self.dt_pin.lock().unwrap();
            dt_pin.set_async_interrupt(Trigger::Both, None, move |event: Event| {
                let old_state = state.load(Ordering::SeqCst);
                let old_direction = direction.load(Ordering::SeqCst);
                if let Ok((new_state, new_direction, trigger)) = Encoder::update_state(
                    old_state,
                    old_direction,
                    Pin::Dt,
                    match event.trigger {
                        Trigger::RisingEdge => 0,
                        Trigger::FallingEdge => 1,
                        _ => return,
                    } as u8,
                ) {
                    state.store(new_state, Ordering::SeqCst);
                    direction.store(new_direction, Ordering::SeqCst);
                    if trigger {
                        callback(&name, new_direction);
                    }
                }
            })?;

            // CLK pin callback setup
            let mut clk_pin = self.clk_pin.lock().unwrap();
            clk_pin.set_async_interrupt(Trigger::Both, None, move |event: Event| {
                let old_state = state_clone.load(Ordering::SeqCst);
                let old_direction = direction_clone.load(Ordering::SeqCst);
                if let Ok((new_state, new_direction, trigger)) = Encoder::update_state(
                    old_state,
                    old_direction,
                    Pin::Clk,
                    match event.trigger {
                        Trigger::RisingEdge => 0,
                        Trigger::FallingEdge => 1,
                        _ => return,
                    } as u8,
                ) {
                    state_clone.store(new_state, Ordering::SeqCst);
                    direction_clone.store(new_direction, Ordering::SeqCst);
                    if trigger {
                        callback(&name_clone, new_direction);
                    }
                }
            })?;

            Ok(())
        }

        // Simulate a clockwise rotation
        fn simulate_clockwise_rotation(&self) {
            // Sequence for clockwise rotation: CLK falls, DT falls, CLK rises, DT rises
            // This simulates 00 -> 10 -> 11 -> 01 -> 00 (rest state)
            let mut clk_pin = self.clk_pin.lock().unwrap();
            clk_pin.simulate_event(Event {
                trigger: Trigger::FallingEdge,
                timestamp: Duration::from_millis(0),
                seqno: 0,
            });
            drop(clk_pin);

            let mut dt_pin = self.dt_pin.lock().unwrap();
            dt_pin.simulate_event(Event {
                trigger: Trigger::FallingEdge,
                timestamp: Duration::from_millis(1),
                seqno: 1,
            });
            drop(dt_pin);

            let mut clk_pin = self.clk_pin.lock().unwrap();
            clk_pin.simulate_event(Event {
                trigger: Trigger::RisingEdge,
                timestamp: Duration::from_millis(2),
                seqno: 2,
            });
            drop(clk_pin);

            let mut dt_pin = self.dt_pin.lock().unwrap();
            dt_pin.simulate_event(Event {
                trigger: Trigger::RisingEdge,
                timestamp: Duration::from_millis(3),
                seqno: 3,
            });
        }

        // Simulate a counter-clockwise rotation
        fn simulate_counter_clockwise_rotation(&self) {
            // Sequence for counter-clockwise rotation: DT falls, CLK falls, DT rises, CLK rises
            // This simulates 00 -> 01 -> 11 -> 10 -> 00 (rest state)
            let mut dt_pin = self.dt_pin.lock().unwrap();
            dt_pin.simulate_event(Event {
                trigger: Trigger::FallingEdge,
                timestamp: Duration::from_millis(0),
                seqno: 0,
            });
            drop(dt_pin);

            let mut clk_pin = self.clk_pin.lock().unwrap();
            clk_pin.simulate_event(Event {
                trigger: Trigger::FallingEdge,
                timestamp: Duration::from_millis(1),
                seqno: 1,
            });
            drop(clk_pin);

            let mut dt_pin = self.dt_pin.lock().unwrap();
            dt_pin.simulate_event(Event {
                trigger: Trigger::RisingEdge,
                timestamp: Duration::from_millis(2),
                seqno: 2,
            });
            drop(dt_pin);

            let mut clk_pin = self.clk_pin.lock().unwrap();
            clk_pin.simulate_event(Event {
                trigger: Trigger::RisingEdge,
                timestamp: Duration::from_millis(3),
                seqno: 3,
            });
        }
    }

    #[test]
    fn test_update_state_clockwise() {
        // Test state transitions for clockwise rotation
        let (new_state, direction, _) = Encoder::update_state(0b00, Direction::None, Pin::Clk, 1).unwrap();
        assert_eq!(new_state, 0b01);
        assert_eq!(direction, Direction::Clockwise);
        
        let (new_state, direction, _) = Encoder::update_state(0b01, Direction::Clockwise, Pin::Dt, 1).unwrap();
        assert_eq!(new_state, 0b11);
        assert_eq!(direction, Direction::Clockwise);
        
        let (new_state, direction, trigger) = Encoder::update_state(0b11, Direction::Clockwise, Pin::Clk, 0).unwrap();
        assert_eq!(new_state, 0b10);
        assert_eq!(direction, Direction::Clockwise);
        assert_eq!(trigger, false); // No trigger yet, this is just an intermediate state
        
        // Test the final transition that should trigger the callback
        let (new_state, direction, trigger) = Encoder::update_state(0b10, Direction::Clockwise, Pin::Dt, 0).unwrap();
        assert_eq!(new_state, 0b00);
        assert_eq!(direction, Direction::Clockwise);
        assert_eq!(trigger, true); // This should trigger the callback
    }
    
    #[test]
    fn test_update_state_counter_clockwise() {
        // Test state transitions for counter-clockwise rotation
        let (new_state, direction, _) = Encoder::update_state(0b00, Direction::None, Pin::Dt, 1).unwrap();
        assert_eq!(new_state, 0b10);
        assert_eq!(direction, Direction::CounterClockwise);
        
        let (new_state, direction, _) = Encoder::update_state(0b10, Direction::CounterClockwise, Pin::Clk, 1).unwrap();
        assert_eq!(new_state, 0b11);
        assert_eq!(direction, Direction::CounterClockwise);
        
        let (new_state, direction, trigger) = Encoder::update_state(0b11, Direction::CounterClockwise, Pin::Dt, 0).unwrap();
        assert_eq!(new_state, 0b01);
        assert_eq!(direction, Direction::CounterClockwise);
        assert_eq!(trigger, false); // No trigger yet, this is just an intermediate state
        
        // Test the final transition that should trigger the callback
        let (new_state, direction, trigger) = Encoder::update_state(0b01, Direction::CounterClockwise, Pin::Clk, 0).unwrap();
        assert_eq!(new_state, 0b00);
        assert_eq!(direction, Direction::CounterClockwise);
        assert_eq!(trigger, true); // This should trigger the callback
    }

    #[test]
    fn test_encoder_rotation_callbacks() {
        // Setup static variables to check callback execution
        static CALLBACK_EXECUTED: AtomicBool = AtomicBool::new(false);
        static DIRECTION: AtomicU8 = AtomicU8::new(0);
        static NAME_MATCHED: AtomicBool = AtomicBool::new(false);
        
        fn test_callback(name: &str, direction: Direction) {
            CALLBACK_EXECUTED.store(true, Ordering::SeqCst);
            NAME_MATCHED.store(name == "test_rotary", Ordering::SeqCst);
            DIRECTION.store(match direction {
                Direction::Clockwise => 1,
                Direction::CounterClockwise => 2,
                Direction::None => 0,
            }, Ordering::SeqCst);
        }
        
        // Create test encoder
        let test_encoder = TestEncoder::new("test_rotary");
        test_encoder.setup(test_callback).unwrap();
        
        // Reset test flags
        CALLBACK_EXECUTED.store(false, Ordering::SeqCst);
        NAME_MATCHED.store(false, Ordering::SeqCst);
        DIRECTION.store(0, Ordering::SeqCst);
        
        // Test clockwise rotation
        test_encoder.simulate_clockwise_rotation();
        
        assert!(CALLBACK_EXECUTED.load(Ordering::SeqCst), "Callback was not executed for clockwise rotation");
        assert!(NAME_MATCHED.load(Ordering::SeqCst), "Encoder name did not match in callback");
        assert_eq!(DIRECTION.load(Ordering::SeqCst), 1, "Direction should be clockwise");
        
        // Reset test flags
        CALLBACK_EXECUTED.store(false, Ordering::SeqCst);
        NAME_MATCHED.store(false, Ordering::SeqCst);
        DIRECTION.store(0, Ordering::SeqCst);
        
        // Test counter-clockwise rotation
        test_encoder.simulate_counter_clockwise_rotation();
        
        assert!(CALLBACK_EXECUTED.load(Ordering::SeqCst), "Callback was not executed for counter-clockwise rotation");
        assert!(NAME_MATCHED.load(Ordering::SeqCst), "Encoder name did not match in callback");
        assert_eq!(DIRECTION.load(Ordering::SeqCst), 2, "Direction should be counter-clockwise");
    }
}

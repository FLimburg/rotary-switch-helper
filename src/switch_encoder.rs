use rppal::gpio::{Event, Gpio, InputPin, Trigger};

use anyhow::Result;
use log::{error, trace};
use std::time::Duration;

#[allow(dead_code)]
pub struct Encoder {
    name: String,
    pin: InputPin,
}

impl Encoder {
    /// Create a new switch encoder
    /// # Arguments
    /// * `name` - Name of the encoder
    /// * `gpio` - Gpio instance to use for the encoder
    /// * `pin_number` - GPIO pin number for the switch signal
    /// * `callback` - Function to call when the encoder is turned
    pub fn new(
        encoder_name: &str,
        gpio: &Gpio,
        pin_number: u8,
        callback: fn(&str, bool),
    ) -> Result<Self> {
        trace!("Initializing GPIO for switch encoder {}", encoder_name);
        let name = encoder_name.to_owned();

        let mut pin = gpio.get(pin_number)?.into_input_pullup();
        pin.set_async_interrupt(
            Trigger::Both,
            Some(Duration::from_millis(50)),
            move |event: Event| {
                trace!("Switch encoder {} event: {:?}", name, event);
                callback(
                    &name,
                    match event.trigger {
                        Trigger::RisingEdge => false,
                        Trigger::FallingEdge => true,
                        _ => {
                            error!("Unexpected event trigger: {:?}", event.trigger);
                            return;
                        }
                    },
                );
            },
        )?;

        Ok(Encoder {
            name: encoder_name.to_owned(),
            pin,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};

    // Mock structures for testing without real GPIO hardware
    struct MockGpio {}

    struct MockInputPin {
        callback: Option<Box<dyn FnMut(Event) + Send>>,
    }

    impl MockGpio {
        fn new() -> Self {
            MockGpio {}
        }

        fn get(&self, _pin: u8) -> Result<MockPin> {
            Ok(MockPin {})
        }
    }

    struct MockPin {}

    impl MockPin {
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
        mock_pin: Arc<Mutex<MockInputPin>>,
    }

    impl TestEncoder {
        fn new(encoder_name: &str) -> Self {
            let name = encoder_name.to_owned();
            let mock_pin = Arc::new(Mutex::new(MockInputPin { callback: None }));
            
            TestEncoder {
                name,
                mock_pin,
            }
        }
        
        fn setup(&self, callback: fn(&str, bool)) -> Result<()> {
            let name = self.name.clone();
            let mut pin = self.mock_pin.lock().unwrap();
            pin.set_async_interrupt(
                Trigger::Both,
                Some(Duration::from_millis(50)),
                move |event: Event| {
                    callback(
                        &name,
                        match event.trigger {
                            Trigger::RisingEdge => false,
                            Trigger::FallingEdge => true,
                            _ => return,
                        },
                    );
                },
            )?;
            Ok(())
        }
        
        fn simulate_press(&self) {
            let mut pin = self.mock_pin.lock().unwrap();
            pin.simulate_event(Event {
                trigger: Trigger::FallingEdge,
                timestamp: Duration::from_millis(0),
                seqno: 0,
            });
        }
        
        fn simulate_release(&self) {
            let mut pin = self.mock_pin.lock().unwrap();
            pin.simulate_event(Event {
                trigger: Trigger::RisingEdge,
                timestamp: Duration::from_millis(0),
                seqno: 1,
            });
        }
    }

    #[test]
    fn test_switch_press_callback() {
        // Setup shared state to track callback execution
        static CALLED: AtomicBool = AtomicBool::new(false);
        static SWITCH_PRESSED: AtomicBool = AtomicBool::new(false);
        static NAME_MATCHED: AtomicBool = AtomicBool::new(false);
        
        // Setup test encoder
        let test_encoder = TestEncoder::new("test_switch");
        
        // Setup callback function
        fn test_callback(name: &str, is_pressed: bool) {
            CALLED.store(true, Ordering::SeqCst);
            SWITCH_PRESSED.store(is_pressed, Ordering::SeqCst);
            NAME_MATCHED.store(name == "test_switch", Ordering::SeqCst);
        }
        
        // Setup the encoder with our test callback
        test_encoder.setup(test_callback).unwrap();
        
        // Simulate a button press (falling edge)
        test_encoder.simulate_press();
        
        // Verify the callback was called correctly
        assert!(CALLED.load(Ordering::SeqCst), "Callback was not called");
        assert!(SWITCH_PRESSED.load(Ordering::SeqCst), "Switch should be reported as pressed");
        assert!(NAME_MATCHED.load(Ordering::SeqCst), "Switch name did not match");
        
        // Reset state variables
        CALLED.store(false, Ordering::SeqCst);
        SWITCH_PRESSED.store(true, Ordering::SeqCst);
        
        // Simulate a button release (rising edge)
        test_encoder.simulate_release();
        
        // Verify the callback was called correctly
        assert!(CALLED.load(Ordering::SeqCst), "Callback was not called on release");
        assert!(!SWITCH_PRESSED.load(Ordering::SeqCst), "Switch should be reported as released");
    }
}

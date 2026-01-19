use rppal::gpio::{Event, Gpio, InputPin, Trigger};

use anyhow::Result;
use log::{error, trace};
use std::time::Duration;

#[allow(dead_code)]
pub struct Encoder {
    name: String,
    name_lp: Option<String>,
    pin: InputPin,
    time_threshold: Option<Duration>,
}

impl Encoder {
    /// Create a new switch encoder
    /// # Arguments
    /// * `encoder_name` - Name of the encoder
    /// * `encoder_name_long_press` - Name of the encoder for long presses
    /// * `gpio` - Gpio instance to use for the encoder
    /// * `pin_number` - GPIO pin number for the switch signal
    /// * `time_threshold`- timer to hold a press before considered a long press
    /// * `callback` - Function to call when the encoder is switched
    pub fn new(
        encoder_name: &str,
        encoder_name_long_press: Option<&str>,
        gpio: &Gpio,
        pin_number: u8,
        time_threshold: Option<Duration>,
        callback: fn(&str, bool),
    ) -> Result<Self> {
        trace!("Initializing GPIO for switch encoder {}", encoder_name);
        let name = encoder_name.to_owned();
        let _name_lp = encoder_name_long_press.map(|s| s.to_owned());

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
            name_lp: encoder_name_long_press.map(|s| s.to_owned()),
            pin,
            time_threshold,
        })
    }
}

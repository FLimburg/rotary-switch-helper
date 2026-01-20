use rppal::gpio::{Event, Gpio, InputPin, Trigger};

use anyhow::{Result, anyhow};
use atomic_time::AtomicOptionDuration;
use log::{error, trace};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

#[allow(dead_code)]
pub struct Encoder {
    name: String,
    name_lp: Option<String>,
    pin: InputPin,
    time_threshold: Option<Duration>,
    last_press: Arc<AtomicOptionDuration>,
    callback: fn(&str, bool),
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

        let pin = gpio.get(pin_number)?.into_input_pullup();

        let mut encoder = Self {
            name: encoder_name.to_owned(),
            name_lp: encoder_name_long_press.map(|s| s.to_owned()),
            pin,
            time_threshold,
            last_press: Arc::new(AtomicOptionDuration::new(None)),
            callback,
        };

        encoder
            .enable_callback()
            .map_err(|e| anyhow!("Failed to enable callbacks: {}", e))?;
        trace!(
            "Switch encoder {}/{:?} initialized",
            encoder.name, encoder.name_lp
        );
        Ok(encoder)
    }

    fn enable_callback(&mut self) -> Result<()> {
        trace!(
            "Enabling callbacks for rotary encoder {}/{:?}",
            self.name, self.name_lp
        );

        let name = self.name.to_owned();
        let last_press = Arc::clone(&self.last_press);
        let time_threshold: Duration = self
            .time_threshold
            .unwrap_or_else(|| Duration::from_secs(0));
        let callback = self.callback;

        match self.name_lp.as_ref() {
            None => {
                self.pin.set_async_interrupt(
                    Trigger::Both,
                    Some(Duration::from_millis(50)),
                    move |event: Event| {
                        trace!("Switch encoder {} event: {:?}", name, event);
                        callback(
                            &name,
                            match event.trigger {
                                Trigger::RisingEdge => false, // release
                                Trigger::FallingEdge => true, // press
                                _ => {
                                    error!("Unexpected event trigger: {:?}", event.trigger);
                                    return;
                                }
                            },
                        );
                    },
                )?;
            }
            Some(name_lp) => {
                let name_lp = name_lp.to_owned();
                self.pin.set_async_interrupt(
                    Trigger::Both,
                    Some(Duration::from_millis(50)),
                    move |event: Event| {
                        let previous_timestamp = last_press.load(Ordering::SeqCst);
                        trace!(
                            "Switch encoder {} event: {:?} (last timestamp {:?})",
                            name, event, previous_timestamp
                        );

                        match event.trigger {
                            // false: release
                            Trigger::RisingEdge => {
                                if let Some(prev_ts) = previous_timestamp
                                    && event.timestamp - prev_ts > time_threshold
                                {
                                    callback(&name_lp, false);
                                } else {
                                    callback(&name, false);
                                }
                                last_press.store(None, Ordering::SeqCst);
                            }
                            // true: press
                            Trigger::FallingEdge => {
                                trace!(
                                    "Storing current time stamp {:?} from seq# {:?}",
                                    event.timestamp, event.seqno
                                );
                                last_press.store(Some(event.timestamp), Ordering::SeqCst);
                                (callback)(&name, true);
                            }
                            _ => {
                                error!("Unexpected event trigger: {:?}", event.trigger);
                            }
                        }
                    },
                )?;
            }
        }

        Ok(())
    }
}

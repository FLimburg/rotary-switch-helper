use std::time::Duration;

use anyhow::Result;
use log::{debug, trace};
use rppal::gpio::Gpio;

pub mod rotary_encoder;
pub mod switch_encoder;

use rotary_encoder::{Direction, Resistor};

#[allow(dead_code)]
pub struct PiInput {
    rot_encoders: Vec<rotary_encoder::Encoder>,
    sw_encoders: Vec<switch_encoder::Encoder>,
}

#[derive(Debug)]
pub enum EncoderType {
    Rotary,
    Switch,
}

#[derive(Debug)]
pub struct SwitchDefinition {
    pub name: String,
    pub name_long_press: Option<String>,
    pub sw_pin: u8,
    pub callback: fn(&str, bool),
    pub time_threshold: Option<Duration>,
}

#[derive(Debug)]
pub struct RotaryDefinition {
    pub name: String,
    pub name_shifted: Option<String>,
    pub sw_pin: Option<u8>,
    pub dt_pin: u8,
    pub clk_pin: u8,
    pub pull_up_down: Resistor,
    pub callback: fn(&str, Direction),
}

impl PiInput {
    pub fn new(switches: &[SwitchDefinition], rotaries: &[RotaryDefinition]) -> Result<Self> {
        debug!("Initializing PiInput...");
        let gpio = Gpio::new()?;

        let rot_encoders = rotaries
            .iter()
            .map(|r| {
                let config =
                    rotary_encoder::EncoderConfig::new(r.dt_pin, r.clk_pin, r.pull_up_down);
                let config = match r.sw_pin {
                    Some(pin) => config.with_switch(pin),
                    None => config,
                };
                rotary_encoder::Encoder::new(
                    &r.name,
                    r.name_shifted.as_deref(),
                    &gpio,
                    config,
                    r.callback,
                )
            })
            .collect::<Result<Vec<rotary_encoder::Encoder>>>()?;

        let sw_encoders = switches
            .iter()
            .map(|s| {
                switch_encoder::Encoder::new(
                    &s.name,
                    s.name_long_press.as_deref(),
                    &gpio,
                    s.sw_pin,
                    s.time_threshold,
                    s.callback,
                )
            })
            .collect::<Result<Vec<switch_encoder::Encoder>>>()?;

        trace!("PiInput initialized");
        Ok(Self {
            rot_encoders,
            sw_encoders,
        })
    }
}

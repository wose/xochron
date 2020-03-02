use embedded_hal::digital::v2::OutputPin;
use nrf52832_hal::gpio::{Output, Pin, PushPull};

type BLPin = Pin<Output<PushPull>>;

pub struct Backlight {
    level: u8,
    low: BLPin,
    mid: BLPin,
    high: BLPin,
}

impl Backlight {
    pub fn new(level: u8, low: BLPin, mid: BLPin, high: BLPin) -> Self {
        let mut backlight = Backlight {
            level: level,
            low,
            mid,
            high,
        };

        backlight.apply();
        backlight
    }

    pub fn on(&mut self) {
        self.apply();
    }

    pub fn off(&mut self) {
        self.set(0);
    }

    pub fn set_level(&mut self, level: u8) {
        if level <= 0b111 {
            self.level = level;
            self.apply();
        }
    }

    pub fn increase(&mut self) {
        if self.level < 0b111 {
            self.level += 1;
            self.apply();
        }
    }

    pub fn decrease(&mut self) {
        if self.level > 0 {
            self.level -= 1;
            self.apply();
        }
    }

    fn apply(&mut self) {
        self.set(self.level);
    }

    fn set(&mut self, level: u8) {
        if level & 0b001 != 0 {
            self.low.set_low().unwrap();
        } else {
            self.low.set_high().unwrap();
        }

        if level & 0b010 != 0 {
            self.mid.set_low().unwrap();
        } else {
            self.mid.set_high().unwrap();
        }

        if level & 0b100 != 0 {
            self.high.set_low().unwrap();
        } else {
            self.high.set_high().unwrap();
        }
    }
}

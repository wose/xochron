use embedded_hal as hal;

use heapless::consts::U256;
use heapless::Vec;

use hrs3300::Hrs3300;

const BUFFER_SIZE: usize = 240;

pub struct HeartRateSensor {
    bpm: Option<u8>,
    /// Raw sensor data
    hr_data: Vec<u32, U256>,
    /// points to the last updated measurement
    index: usize,
    /// is the sensor measuring
    measuring: bool,
    /// minimum raw data
    min: u32,
    /// maximum raw data
    max: u32,
}

impl HeartRateSensor {
    pub fn new() -> Result<Self, ()> {
        let mut buffer: Vec<u32, U256> = Vec::new();
        buffer.resize_default(BUFFER_SIZE)?;

        Ok(Self {
            bpm: None,
            hr_data: buffer,
            index: 0,
            measuring: false,
            min: 0xFFFF_FFFF,
            max: 0,
        })
    }

    pub fn update_hrs<I2C, E>(&mut self, hrs: &mut Hrs3300<I2C>) -> Result<(), hrs3300::Error<E>>
    where
        I2C: hal::blocking::i2c::Write<Error = E> + hal::blocking::i2c::WriteRead<Error = E>,
    {
        if !self.measuring {
            hrs.enable_hrs()?;
            hrs.enable_oscillator()?;
            self.measuring = true;
        }

        self.index = if self.index < BUFFER_SIZE - 1 {
            self.index + 1
        } else {
            0
        };

        let value = hrs.read_hrs()?;
        self.update_value(value);

        Ok(())
    }

    fn update_value(&mut self, value: u32) {
        if value > self.max {
            self.max = value;
        }
        if value < self.min {
            self.min = value;
        }

        self.hr_data[self.index] = value;
    }

    pub fn values<'a>(&'a self) -> &'a [u32] {
        &self.hr_data
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn value(&self) -> (usize, u32) {
        (self.index, self.hr_data[self.index])
    }

    pub fn value_norm(&self) -> (usize, f32) {
        if self.min >= self.max {
            (0, 0.0)
        } else {
            (
                self.index,
                (self.hr_data[self.index] - self.min) as f32 / (self.max - self.min) as f32,
            )
        }
    }

    pub fn prev_value(&self) -> (usize, u32) {
        let prev_index = match self.index {
            0 => BUFFER_SIZE - 1,
            index => index - 1,
        };
        (prev_index, self.hr_data[prev_index])
    }

    pub fn prev_value_norm(&self) -> (usize, f32) {
        let (prev_index, prev_value) = self.prev_value();
        (
            prev_index,
            (prev_value - self.min) as f32 / (self.max - self.min) as f32,
        )
    }

    pub fn bpm(&self) -> Option<u8> {
        self.bpm
    }

    pub fn min(&self) -> u32 {
        self.min
    }

    pub fn max(&self) -> u32 {
        self.max
    }
}

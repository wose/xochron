#![no_std]
#![no_main]

#[allow(unused_imports)]
use panic_semihosting;

use cortex_m_semihosting::hprintln;

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use embedded_hal::digital::v2::OutputPin;
use hal::gpio::Level;
use hal::prelude::GpioExt;
use hal::spim;
use nrf52832_hal as hal;

use st7735_lcd::{Orientation, ST7735};

use rtfm::app;

#[app(device = crate::hal::target, peripherals = true)]
const APP: () = {
    #[init]
    fn init(cx: init::Context) {
        hprintln!("init").unwrap();

        let mut delay = hal::Delay::new(cx.core.SYST);

        let port0 = cx.device.P0.split();

        let mut backlight = port0.p0_22.into_push_pull_output(Level::Low);
        let rst = port0.p0_26.into_push_pull_output(Level::Low);
        let _cs = port0.p0_25.into_push_pull_output(Level::Low);
        let dc = port0.p0_18.into_push_pull_output(Level::Low);

        let spi_clk = port0.p0_02.into_push_pull_output(Level::Low).degrade();
        let spi_mosi = port0.p0_03.into_push_pull_output(Level::Low).degrade();

        let pins = spim::Pins {
            sck: spi_clk,
            miso: None,
            mosi: Some(spi_mosi),
        };

        let spi = spim::Spim::new(
            cx.device.SPIM0,
            pins,
            spim::Frequency::M8,
            spim::MODE_3,
            122,
        );

        let mut display = ST7735::new(spi, dc, rst, true, true);
        display.init(&mut delay).unwrap();
        display.set_orientation(&Orientation::Portrait).unwrap();

        let bg = (33, 33, 33);
        let blank = Rectangle::new(Coord::new(0, 0), Coord::new(239, 239)).fill(Some(bg.into()));

        display.draw(blank);

        backlight.set_high().unwrap();
        cx.device.POWER.systemoff.write(|w| w.systemoff().set_bit());
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        hprintln!("idle").unwrap();

        loop {}
    }
};

#![no_std]
#![no_main]

#[allow(unused_imports)]
use panic_semihosting;

use cortex_m_semihosting::hprintln;

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use embedded_hal::digital::v2::OutputPin;
use hal::gpio::{Input, Level, Pin, PullUp};
use hal::prelude::GpioExt;
use hal::spim;
use nrf52832_hal as hal;

use st7735_lcd::{Orientation, ST7735};

use rtfm::app;

#[app(device = crate::hal::target, peripherals = true)]
const APP: () = {
    struct Resources {
        button: Pin<Input<PullUp>>,
        gpiote: hal::target::GPIOTE,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        hprintln!("init").unwrap();

        let mut delay = hal::Delay::new(cx.core.SYST);

        let port0 = cx.device.P0.split();

        let _enable_button = port0.p0_15.into_push_pull_output(Level::Low);
        let _backlight = port0.p0_22.into_push_pull_output(Level::Low);
        let button = port0.p0_13.into_pullup_input().degrade();
        let _touch = port0.p0_28.into_pullup_input();
        let _bma = port0.p0_08.into_pullup_input();
        let _charging = port0.p0_12.into_pullup_input();

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

        let bg = (255, 255, 0);
        let blank = Rectangle::new(Coord::new(0, 0), Coord::new(9, 9)).fill(Some(bg.into()));

        display.draw(blank);

        // Channel 0 - Button down
        cx.device
            .GPIOTE
            .config
            .iter()
            .nth(0)
            .unwrap()
            .write(|w| unsafe { w.mode().event().polarity().hi_to_lo().psel().bits(13) });

        cx.device.GPIOTE.intenset.write(|w| w.in0().set_bit());

        // Channel 1 - Button up
        cx.device
            .GPIOTE
            .config
            .iter()
            .nth(1)
            .unwrap()
            .write(|w| unsafe { w.mode().event().polarity().lo_to_hi().psel().bits(13) });

        cx.device.GPIOTE.intenset.write(|w| w.in1().set_bit());

        // Channel 2 - Touch Event
        cx.device
            .GPIOTE
            .config
            .iter()
            .nth(2)
            .unwrap()
            .write(|w| unsafe { w.mode().event().polarity().lo_to_hi().psel().bits(28) });

        cx.device.GPIOTE.intenset.write(|w| w.in2().set_bit());

        // Channel 3 - Accelerometer Event
        cx.device
            .GPIOTE
            .config
            .iter()
            .nth(3)
            .unwrap()
            .write(|w| unsafe { w.mode().event().polarity().lo_to_hi().psel().bits(8) });

        cx.device.GPIOTE.intenset.write(|w| w.in3().set_bit());

        // Channel 4 - Charging on
        cx.device
            .GPIOTE
            .config
            .iter()
            .nth(4)
            .unwrap()
            .write(|w| unsafe { w.mode().event().polarity().hi_to_lo().psel().bits(12) });

        cx.device.GPIOTE.intenset.write(|w| w.in4().set_bit());

        // Channel 5 - Charging off
        cx.device
            .GPIOTE
            .config
            .iter()
            .nth(5)
            .unwrap()
            .write(|w| unsafe { w.mode().event().polarity().lo_to_hi().psel().bits(12) });

        cx.device.GPIOTE.intenset.write(|w| w.in5().set_bit());

        init::LateResources {
            button,
            gpiote: cx.device.GPIOTE,
        }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        hprintln!("idle").unwrap();

        loop {
            continue;
        }
    }

    #[task(binds = GPIOTE, resources = [gpiote])]
    fn gpiote(cx: gpiote::Context) {
        let gpiote = cx.resources.gpiote;

        // Channel 0 - Button down
        if gpiote.events_in.iter().nth(0).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(0).unwrap().reset();
            hprintln!("Button down").unwrap();
        }

        // Channel 1 - Button up
        if gpiote.events_in.iter().nth(1).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(1).unwrap().reset();
            hprintln!("Button up").unwrap();
        }

        // Channel 2 - Touch Event
        if gpiote.events_in.iter().nth(2).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(2).unwrap().reset();
            hprintln!("A Touch!").unwrap();
        }

        // Channel 3 - Accelerometer Event
        if gpiote.events_in.iter().nth(3).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(3).unwrap().reset();
            hprintln!("Accelerometer").unwrap();
        }

        // Channel 4 - Charging on
        if gpiote.events_in.iter().nth(4).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(4).unwrap().reset();
            hprintln!("Charging on").unwrap();
        }

        // Channel 5 - Charging off
        if gpiote.events_in.iter().nth(5).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(5).unwrap().reset();
            hprintln!("Charging off").unwrap();
        }
    }
};

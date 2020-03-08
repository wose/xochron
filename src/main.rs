#![no_std]
#![no_main]

#[allow(unused_imports)]
use panic_semihosting;

use chrono::{NaiveDateTime, Timelike};
use cortex_m_semihosting::hprintln;

use embedded_graphics::{egtext, fonts::Font24x32, pixelcolor::Rgb565, prelude::*, text_style};

use hal::{
    gpio::{Input, Level, Output, Pin, PullUp, PushPull},
    prelude::{ClocksExt, GpioExt, RtcExt},
    spim,
};
use heapless::{consts::U8, String};
use nrf52832_hal as hal;

use rtfm::app;
use st7789v::ST7789V;
use ufmt::uwrite;
use ufmt_utils::WriteAdapter;

mod backlight;
use backlight::Backlight;

macro_rules! gpiote_event {
    ($cx:expr, $pin:literal, $chan:literal, $evin:ident, $pol:ident) => {
        $cx.device
            .GPIOTE
            .config
            .iter()
            .nth($chan)
            .unwrap()
            .write(|w| unsafe { w.mode().event().polarity().$pol().psel().bits($pin) });

        $cx.device.GPIOTE.intenset.write(|w| w.$evin().set_bit());
    };
}

type Display = ST7789V<
    spim::Spim<hal::nrf52832_pac::SPIM0>,
    Pin<Output<PushPull>>,
    Pin<Output<PushPull>>,
    Pin<Output<PushPull>>,
    (),
    hal::spim::Error,
>;

#[app(device = crate::hal::target, peripherals = true)]
const APP: () = {
    struct Resources {
        backlight: Backlight,
        button: Pin<Input<PullUp>>,
        display: Display,
        gpiote: hal::target::GPIOTE,
        datetime: NaiveDateTime,
        rtc: hal::Rtc<hal::target::RTC0, hal::rtc::Started>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        hprintln!("init").unwrap();

        let mut delay = hal::Delay::new(cx.core.SYST);

        let port0 = cx.device.P0.split();
        let mut nvic = cx.core.NVIC;

        let clocks = ClocksExt::constrain(cx.device.CLOCK);
        clocks
            // use external 32MHz oscillator
            .enable_ext_hfosc()
            // use external 32.768 kHz crystal oscillator
            .set_lfclk_src_external(hal::clocks::LfOscConfiguration::NoExternalNoBypass)
            .start_lfclk();

        let mut rtc = RtcExt::constrain(cx.device.RTC0);
        // Tick ebery 1/8 s
        rtc.set_prescaler(0xFFF).unwrap();
        let mut rtc = rtc.enable_counter();
        rtc.enable_interrupt(hal::rtc::RtcInterrupt::Tick, &mut nvic);

        let _enable_button = port0.p0_15.into_push_pull_output(Level::Low);

        // interrupt pins
        let button = port0.p0_13.into_pullup_input().degrade();
        let _touch = port0.p0_28.into_pullup_input();
        let _bma = port0.p0_08.into_pullup_input();
        let _charging = port0.p0_12.into_pullup_input();
        let _power = port0.p0_19.into_pullup_input();

        // Backlight
        let backlight_low = port0.p0_14.into_push_pull_output(Level::Low).degrade();
        let backlight_mid = port0.p0_22.into_push_pull_output(Level::Low).degrade();
        let backlight_high = port0.p0_23.into_push_pull_output(Level::Low).degrade();

        // display
        let rst = port0.p0_26.into_push_pull_output(Level::Low).degrade();
        let cs = port0.p0_25.into_push_pull_output(Level::Low).degrade();
        let dc = port0.p0_18.into_push_pull_output(Level::Low).degrade();

        // spi
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

        let mut display = ST7789V::with_cs(spi, cs, dc, rst).unwrap();
        display.init(&mut delay).unwrap();

        // Channel 0 - Button down
        gpiote_event!(cx, 13, 0, in0, hi_to_lo);
        // Channel 1 - Button up
        gpiote_event!(cx, 13, 1, in1, lo_to_hi);
        // Channel 2 - Touch Event
        gpiote_event!(cx, 28, 2, in2, lo_to_hi);
        // Channel 3 - Accelerometer Event
        gpiote_event!(cx, 8, 3, in3, lo_to_hi);
        // Channel 4 - Charging on
        gpiote_event!(cx, 12, 4, in4, hi_to_lo);
        // Channel 5 - Charging off
        gpiote_event!(cx, 12, 5, in5, lo_to_hi);
        // Channel 6 - Power connected
        gpiote_event!(cx, 19, 6, in6, hi_to_lo);
        // Channel 7 - Power disconnected
        gpiote_event!(cx, 19, 7, in7, lo_to_hi);

        let backlight = Backlight::new(0b010, backlight_low, backlight_mid, backlight_high);
        display.clear(Rgb565::BLACK).unwrap();

        init::LateResources {
            backlight,
            button,
            datetime: NaiveDateTime::from_timestamp(0, 0),
            display,
            gpiote: cx.device.GPIOTE,
            rtc,
        }
    }

    #[idle(spawn = [update_ui])]
    fn idle(cx: idle::Context) -> ! {
        hprintln!("idle").unwrap();

        loop {
            cx.spawn.update_ui().unwrap();
            continue;
        }
    }

    #[task(resources = [datetime, display])]
    fn update_ui(cx: update_ui::Context) {
        //        hprintln!("Update UI").unwrap();
        let mut datetime = cx.resources.datetime;

        let mut buf: String<U8> = String::new();
        datetime.lock(|datetime| {
            uwrite!(
                WriteAdapter(&mut buf),
                "{}:{}",
                datetime.hour(),
                datetime.minute()
            )
            .unwrap();
        });

        let display = cx.resources.display;

        egtext!(
            text = &buf,
            top_left = (60, 44),
            style = text_style!(
                font = Font24x32,
                text_color = Rgb565::WHITE,
                background_color = Rgb565::BLACK,
            )
        )
        .draw(display)
        .unwrap();
    }

    #[task(binds = RTC0, priority = 2, resources = [datetime, rtc])]
    fn rtc0(cx: rtc0::Context) {
        if cx
            .resources
            .rtc
            .get_event_triggered(hal::rtc::RtcInterrupt::Tick, true)
            && cx.resources.rtc.get_counter() & 0b111 == 0b111
        {
            *cx.resources.datetime += chrono::Duration::seconds(1);
        }
    }

    #[task(binds = GPIOTE, resources = [gpiote, backlight])]
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
            cx.resources.backlight.decrease();
        }

        // Channel 2 - Touch Event
        if gpiote.events_in.iter().nth(2).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(2).unwrap().reset();
            hprintln!("A Touch!").unwrap();
            cx.resources.backlight.increase();
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

        // Channel 6 - Power connected
        if gpiote.events_in.iter().nth(6).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(6).unwrap().reset();
            hprintln!("Power connected").unwrap();
            cx.resources.backlight.on();
        }

        // Channel 7 - Power disconnected
        if gpiote.events_in.iter().nth(7).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(7).unwrap().reset();
            hprintln!("Power disconnected").unwrap();
            cx.resources.backlight.off();
        }
    }

    // Interrupt handleres used to dispatch software tasks
    extern "C" {
        fn SWI0_EGU0();
    }
};

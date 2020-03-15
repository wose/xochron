#![no_std]
#![no_main]

#[allow(unused_imports)]
use panic_semihosting;

use chrono::{NaiveDateTime, Timelike};
use cortex_m_semihosting::{hprint, hprintln};

use embedded_graphics::{
    egline, egtext, fonts::Font24x32, pixelcolor::Rgb565, prelude::*, primitive_style, text_style,
};

use hal::{
    gpio::{Input, Level, Output, Pin, PullUp, PushPull},
    prelude::{ClocksExt, GpioExt, RtcExt},
    spim, twim,
};
use heapless::{consts::U8, String};
use hrs3300::{ConversionDelay, Hrs3300, LedCurrent};
use nrf52832_hal as hal;

use rtfm::app;
use st7789v::ST7789V;
use ufmt::uwrite;
use ufmt_utils::WriteAdapter;

mod backlight;
use backlight::Backlight;

mod sensors;
use sensors::HeartRateSensor;

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

pub struct State {
    running: bool,
    heart_rate_sensor: HeartRateSensor,
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
        datetime: NaiveDateTime,
        delay: hal::Delay,
        display: Display,
        gpiote: hal::target::GPIOTE,
        i2c: Option<twim::Twim<hal::target::TWIM1>>,
        rtc: hal::Rtc<hal::target::RTC0, hal::rtc::Started>,
        state: State,
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
        // Tick ebery 1/128 s
        rtc.set_prescaler(255).unwrap();
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

        // i2c
        let scl = port0.p0_07.into_floating_input().degrade();
        let sda = port0.p0_06.into_floating_input().degrade();

        let pins = twim::Pins { scl, sda };
        let i2c = twim::Twim::new(cx.device.TWIM1, pins, twim::Frequency::K400);

        let mut display = ST7789V::with_cs(spi, cs, dc, rst).unwrap();
        display.init(&mut delay).unwrap();

        let mut hrs = Hrs3300::new(i2c);
        hrs.init().unwrap();
        //hrs.enable_hrs().unwrap();
        //hrs.enable_oscillator().unwrap();
        hrs.set_conversion_delay(ConversionDelay::Ms12_5).unwrap();
        hrs.set_led_current(LedCurrent::Ma40).unwrap();
        //hrs.set_gain(hrs3300::Gain::Eight).unwrap();
        let i2c = hrs.destroy();

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
        let state = State {
            running: false,
            heart_rate_sensor: HeartRateSensor::new(),
        };

        init::LateResources {
            backlight,
            button,
            datetime: NaiveDateTime::from_timestamp(0, 0),
            delay,
            display,
            gpiote: cx.device.GPIOTE,
            i2c: Some(i2c),
            rtc,
            state,
        }
    }

    #[idle(spawn = [update_ui, measure_hr], resources = [delay, i2c])]
    fn idle(cx: idle::Context) -> ! {
        hprintln!("idle").unwrap();

        loop {
            continue;
        }
    }

    #[task(resources = [datetime, display, state])]
    fn update_ui(cx: update_ui::Context) {
        let display = cx.resources.display;

        if true {
            let mut state = cx.resources.state;
            state.lock(|state| {
                let index = state.heart_rate_sensor.index() as i32;

                egline!(
                    start = (index, 0i32),
                    end = (index, 239i32),
                    style = primitive_style!(stroke_width = 1, stroke_color = Rgb565::BLACK),
                )
                .draw(display)
                .unwrap();

                let range = (state.heart_rate_sensor.max() - state.heart_rate_sensor.min()) as f32;

                if range > 0.0 {
                    let hrs = &state.heart_rate_sensor;
                    let (index, value_norm) = hrs.value_norm();
                    let index = index as i32;
                    let scaled_value = (value_norm * 239.0) as i32;

                    if index == 0 {
                        embedded_graphics::drawable::Pixel(
                            Point::new(index, scaled_value),
                            Rgb565::WHITE,
                        )
                        .draw(display)
                        .unwrap();
                    } else {
                        let (prev_index, prev_value_norm) = hrs.prev_value_norm();
                        let prev_scaled_value = (prev_value_norm * 239.0) as i32;
                        egline!(
                            start = (prev_index as i32, prev_scaled_value),
                            end = (index, scaled_value),
                            style =
                                primitive_style!(stroke_width = 1, stroke_color = Rgb565::WHITE),
                        )
                        .draw(display)
                        .unwrap();
                    };
                }
            });
        } else {
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
    }

    #[task(priority = 2, resources = [i2c, rtc, state])]
    fn measure_hr(cx: measure_hr::Context) {
        static mut started: bool = false;

        if let Some(i2c) = cx.resources.i2c.take() {
            let mut state = cx.resources.state;
            let mut hrs = Hrs3300::new(i2c);
            if !*started {
                hrs.enable_hrs().unwrap();
                hrs.enable_oscillator().unwrap();
                *started = true;
            }
            let hrs_raw = hrs.read_hrs().unwrap();
            state.lock(|state| state.heart_rate_sensor.update_hrs(hrs_raw));
            *cx.resources.i2c = Some(hrs.destroy());
        }
    }

    #[task(binds = RTC0, priority = 3, spawn = [measure_hr, update_ui], resources = [datetime, rtc, state])]
    fn rtc0(cx: rtc0::Context) {
        if cx
            .resources
            .rtc
            .get_event_triggered(hal::rtc::RtcInterrupt::Tick, true)
        {
            let counter = cx.resources.rtc.get_counter();
            if counter & 0x7F == 0x7F {
                *cx.resources.datetime += chrono::Duration::seconds(1);
            }

            //if cx.resources.state.running && counter & 10 == 10 {
            if counter & 0x01 == 0x01 {
                cx.spawn.measure_hr().ok();
                cx.spawn.update_ui().ok();
            }
        }
    }

    #[task(binds = GPIOTE, resources = [backlight, gpiote, state])]
    fn gpiote(cx: gpiote::Context) {
        let gpiote = cx.resources.gpiote;

        // Channel 0 - Button down
        if gpiote.events_in.iter().nth(0).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(0).unwrap().reset();
            //hprintln!("Button down").unwrap();
        }

        // Channel 1 - Button up
        if gpiote.events_in.iter().nth(1).unwrap().read().bits() != 0 {
            gpiote.events_in.iter().nth(1).unwrap().reset();
            //hprintln!("Button up").unwrap();
            let mut state = cx.resources.state;
            state.lock(|state| state.running = true);
            //cx.resources.backlight.decrease();
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
        fn SWI1_EGU1();
    }
};

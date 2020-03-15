use chrono::{NaiveDateTime, Timelike};
use embedded_graphics::{
    drawable::Drawable,
    egtext,
    fonts::Font24x32,
    pixelcolor::{BinaryColor, PixelColor, Rgb565},
    prelude::*,
    text_style,
};
use heapless::{consts::U8, String};
use ufmt::uwrite;
use ufmt_utils::WriteAdapter;

macro_rules! dtfmt {
    ($buf: expr, $fmt:literal, $h:expr, $m:expr) => {
        uwrite!(WriteAdapter(&mut $buf), $fmt, $h, $m,).unwrap();
    };
}

pub struct DigitalClock<'a, C: PixelColor> {
    bg_color: C,
    datetime: &'a NaiveDateTime,
    fg_color: C,
}

impl<'a> DigitalClock<'a, Rgb565> {
    pub fn with_date(datetime: &'a NaiveDateTime) -> Self {
        DigitalClock {
            bg_color: Rgb565::BLACK,
            datetime,
            fg_color: Rgb565::WHITE,
        }
    }

    pub fn fg_color(&mut self, fg_color: Rgb565) {
        self.fg_color = fg_color;
    }

    pub fn bg_color(&mut self, bg_color: Rgb565) {
        self.bg_color = bg_color;
    }
}

impl<'a, C: 'a> Drawable<C> for &DigitalClock<'a, C>
where
    C: PixelColor + From<BinaryColor>,
{
    fn draw<D>(self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<C>,
    {
        let mut buf: String<U8> = String::new();
        let hour = self.datetime.hour();
        let minute = self.datetime.minute();

        // add leading zeroes
        match (hour, minute) {
            (h, m) if h <= 9 && m <= 9 => dtfmt!(buf, "0{}:0{}", h, m),
            (h, m) if h <= 9 => dtfmt!(buf, "0{}:{}", h, m),
            (h, m) if m <= 9 => dtfmt!(buf, "{}:0{}", h, m),
            (h, m) => dtfmt!(buf, "{}:{}", h, m),
        };

        egtext!(
            text = &buf,
            top_left = (60, 40),
            style = text_style!(
                font = Font24x32,
                text_color = self.fg_color,
                background_color = self.bg_color,
            )
        )
        .draw(display)
    }
}

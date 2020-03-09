# XOchron

A [embedded-hal] and [rtfm] powered PineTime firmware.

> The XO is typically responsible for the management of day-to-day activities,
> freeing the commander to concentrate on strategy and planning the unit's next
> move. - [Wikipedia]

[embedded-hal]: https://github.com/rust-embedded/embedded-hal
[rtfm]: https://rtfm.rs/0.5/book/en/
[Wikipedia]: https://en.wikipedia.org/wiki/Executive_officer

## Hacking

Clone, build and flash the firmware. I use a ST-LINK/V2 clone and openocd to
upload the binary.

``` sh
git clone https://github.com/wose/xochron.git
cd xochron
cargo build --release
# start openocd in another terminal after connecting your STLink to your pinewatch
openocd -f interface/stlink-v2.cfg -f target/nrf52.cfg
# start gdb, it will connect to openocd upload and run the firmware
arm-none-eabi-gdb target/thumbv7em-none-eabihf/release/xochron
```

## Hardware

| ic               | datasheet                     | driver crate                      |
|------------------|-------------------------------|-----------------------------------|
| nRF52832         | [Product Brief (PDF)]         | [nrf52832-hal]                    |
|                  | [Product Specification (PDF)] |                                   |
| ST7789V          | [ST7789V (PDF)]               | [st7735-lcd], [st7789], [st7789v] |
| XTX XT25F32B     | similar [Macronix (PDF)]      |                                   |
| Hynitron CST816S | [CST816S EN (PDF)]            |                                   |
| BMA421           | [BMA400 (PDF)]                |                                   |
| HRS3300          | [HRS3300 (PDF)]               | [hrs3300-rs]                      |

You can find more detailed information in the [pine64 wiki].

[BMA400 (PDF)]: https://wiki.pine64.org/images/c/cc/Bst-bma400-ds000.pdf
[CST816S EN (PDF)]: https://wiki.pine64.org/images/5/51/CST816S%E6%95%B0%E6%8D%AE%E6%89%8B%E5%86%8CV1.1.en.pdf
[HRS3300 (PDF)]: http://files.pine64.org/doc/datasheet/pinetime/HRS3300%20Heart%20Rate%20Sensor.pdf
[Macronix (PDF)]: https://www.macronix.com/Lists/Datasheet/Attachments/7426/MX25L3233F,%203V,%2032Mb,%20v1.6.pdf
[nrf52832-hal]: https://crates.io/crates/nrf52832-hal
[pine64 wiki]: https://wiki.pine64.org/index.php/PineTime
[Product Brief (PDF)]: http://files.pine64.org/doc/datasheet/pinetime/nRF52832%20product%20brief.pdf
[Product Specification (PDF)]: https://infocenter.nordicsemi.com/pdf/nRF52832_PS_v1.4.pdf
[st7735-lcd]:https://crates.io/crates/st7735-lcd 
[st7789]: https://crates.io/crates/st7789
[st7789v]: https://github.com/wose/st7789v
[ST7789V (PDF)]: https://wiki.pine64.org/images/5/54/ST7789V_v1.6.pdf
[hrs3300-rs]: https://github.com/eldruin/hrs3300-rs

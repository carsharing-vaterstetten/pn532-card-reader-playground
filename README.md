# pn532 card reader playground

This is a playground for experimenting with the PN532 card reader chip, using a Waveshare test board.

It combines the following components:

* Waveshare PN532 board
* STM32L432/Nucleo 32 developer board
* Rust/Embassy

## Current state and open items

* [x] read UID of cards
* Do some more advanced authentication
* Backend communication
* Reduce power consumption

## Running locally

### Hardware setup

**TODO:** This section needs some more work!

* Put the waveshare board into SPI mode
* Wire up the STM32L432 on a breadboard
    * SCK - PA6 - A5
    * MOSI - PA12 - D2
    * MISO - PA11 - D10
    * NSS - PB0 - D3
    * D20/RST - ?? - D9

### Prepare Rust

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Flash the firmware

```shell
cargo run --release
```

### Result

If all goes well, putting an NFC tag on the card reader should show something like this:


```
    Finished release [optimized + debuginfo] target(s) in 0.88s
     Running `probe-run --chip STM32L432KCUx target/thumbv7em-none-eabi/release/vat-card-reader`
(HOST) INFO  flashing program (19 pages / 19.00 KiB)
(HOST) INFO  success!
────────────────────────────────────────────────────────────────────────────────
0.000152 INFO  Performing reset ...
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:80
0.700744 INFO  Performing reset ... done!
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:87
0.701110 INFO  Sending wakup
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:91
1.701721 INFO  Run
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:97
1.809204 INFO  Firmware: FirmwareVersion { ic: 50, version: 1, revision: 6, supports_iso18092: true, supports_iso14443_a: true, supports_iso14443_b: true }
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:108
4.034881 INFO  Card: CardUid([1, 35, 69, 103, 0, 0, 0])
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:115
4.064697 INFO  Card: CardUid([1, 35, 69, 103, 0, 0, 0])
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:115
4.094512 INFO  Card: CardUid([1, 35, 69, 103, 0, 0, 0])
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:115
4.124328 INFO  Card: CardUid([1, 35, 69, 103, 0, 0, 0])
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:115
4.154113 INFO  Card: CardUid([1, 35, 69, 103, 0, 0, 0])
└─ vat_card_reader::____embassy_main_task::{async_fn#0} @ src/main.rs:115
```
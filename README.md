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

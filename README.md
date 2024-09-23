# embedded-nrf24l01-async

## Features

* Designed for use with the [embedded-hal-async] crate
* Safe and declarative register definitions
* Chip operation modes lifted to the type-level
* Lets you go straight into RX/TX with the default config

## Reference datasheets

* [nRF24L01+](https://www.sparkfun.com/datasheets/Components/SMD/nRF24L01Pluss_Preliminary_Product_Specification_v1_0.pdf)

## Usage

### Parameters

Get the `*-hal` crate for your micro-controller unit. Figure out how
to get to the peripherals implementing these [embedded-hal] traits:

* `embedded_hal::spi::SpiDevice` for the SPI peripheral

  We provide a `mod setup` with a few constants for SPI.
 
* `embedded_hal::digital::OutputPin` for the **CE** pin

### Constructor

```rust
let mut nrf24 = NRF24L01::new(ce, spi).await.unwrap();
```

This will provide an instance of `Standby` 
and activate acknoladgements and dynamic payload length. You can use 
`.rx()` or `.tx()` to transfer into a `RXMode` and `TXMode` instances. They
implement `.standby()` methods to get back to `Standby` and then
switch to the other mode.


### Configuration

Before you start transmission, the device must be configured. Example with an **nrf24l01+**:

```rust
nrf24.set_frequency(8).await.unwrap();
nrf24.set_auto_retransmit(15, 15).await.unwrap();
nrf24.set_rf(&DataRate::R2Mbps, 0).await.unwrap();
nrf24
    .set_pipes_rx_enable(&[true, false, false, false, false, false])
    .await
    .unwrap();
nrf24
    .set_auto_ack(&[true, false, false, false, false, false])
    .await
    .unwrap();
nrf24.set_pipes_rx_lengths(&[None; 6]).await.unwrap();
nrf24.set_crc(CrcMode::TwoBytes).await.unwrap();
nrf24.set_rx_addr(0, &b"fnord"[..]).await.unwrap();
nrf24.set_tx_addr(&b"fnord"[..]).await.unwrap();
nrf24.flush_rx().await.unwrap();
nrf24.flush_tx().await.unwrap();
```

### `RXMode`

Use `rx.can_read()` to poll, then `rx.read()` to receive payload.

If `can_read()` always returns `true`, it's usually a power supply issue.
You can attach a 1uF or 10uF capacitor as close to the module as possible or 
upgrade to a better 3.3v regulator.

### `TXMode`

1. Use `tx.can_send()` to prevent sending on a full queue. Note: not needed if `poll_send` or `wait_empty` was used after `send`. 
1. Use `tx.send()` to enqueue a packet.
1. Use `tx.wait_empty()` to synchronously flush. Or `tx.poll_send()` to asynchronously flush and get whether package transmission was successful.

### Note

Automatic retransmission (for TX) and acknowledgement (for RX) features go hand in hand. Since setting retransmissions means TX device is expecting an ack, and auto acknowledgement means RX device will check if received packet isn't a duplicate + send an ack back. Turn them both on for `tx.poll_send()` to be reliable.


[embedded-hal]: https://crates.io/crates/embedded-hal

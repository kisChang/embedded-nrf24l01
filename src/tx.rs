use crate::command::{Command, FlushTx, WriteTxPayload};
use crate::config::Configuration;
use crate::device::Device;
use crate::registers::{FifoStatus, ObserveTx, Status};
use crate::standby::StandbyMode;
use core::fmt;

/// Represents **TX Mode** and the associated **TX Settling** and
/// **Standby-II** states
///
/// # Timing
///
/// The datasheet states the follwing:
///
/// > It is important to never keep the nRF24L01 in TX mode for more than 4ms at a time.
///
/// No effects have been observed when exceeding this limit. The
/// warranty could get void.
pub struct TxMode<D: Device> {
    device: D,
}

impl<D: Device> fmt::Debug for TxMode<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TxMode")
    }
}

impl<D: Device> TxMode<D> {
    /// Relies on everything being set up by `StandbyMode::tx()`, from
    /// which it is called
    pub(crate) fn new(device: D) -> Self {
        TxMode { device }
    }

    /// Disable `CE` so that you can switch into RX mode.
    pub async fn standby(mut self) -> Result<StandbyMode<D>, D::Error> {
        self.wait_empty().await?;

        Ok(StandbyMode::from_rx_tx(self.device))
    }

    /// Is TX FIFO empty?
    pub async fn is_empty(&mut self) -> Result<bool, D::Error> {
        let (_, fifo_status) = self.device.read_register::<FifoStatus>().await?;
        Ok(fifo_status.tx_empty())
    }

    /// Is TX FIFO full?
    pub async fn is_full(&mut self) -> Result<bool, D::Error> {
        let (_, fifo_status) = self.device.read_register::<FifoStatus>().await?;
        Ok(fifo_status.tx_full())
    }

    /// Does the TX FIFO have space?
    pub async fn can_send(&mut self) -> Result<bool, D::Error> {
        let full = self.is_full().await?;
        Ok(!full)
    }

    /// Send asynchronously
    pub async fn send(&mut self, packet: &[u8]) -> Result<Status, D::Error> {
        let state = self.device.send_command(&WriteTxPayload::new(packet)).await?;
        self.device.ce_enable();
        Ok(state.0)
    }

    /// Poll completion of one or multiple send operations and check whether transmission was
    /// successful.
    ///
    /// This function behaves like `wait_empty()`, except that it returns whether sending was
    /// successful and that it provides an asynchronous interface.
    ///
    /// Automatic retransmission (set_auto_retransmit) and acks (set_auto_ack) have to be
    /// enabled if you actually want to know if transmission was successful. 
    /// Else the nrf24 just transmits the packet once and assumes it was received.
    pub async fn poll_send(&mut self) -> nb::Result<bool, D::Error> {
        let (status, fifo_status) = self.device.read_register::<FifoStatus>().await?;
        // We need to clear all the TX interrupts whenever we return Ok here so that the next call
        // to poll_send correctly recognizes max_rt and send completion.
        if status.max_rt() {
            // If MAX_RT is set, the packet is not removed from the FIFO, so if we do not flush
            // the FIFO, we end up in an infinite loop
            self.device.send_command(&FlushTx).await?;
            self.clear_interrupts_and_ce().await?;
            Ok(false)
        } else if fifo_status.tx_empty() {
            self.clear_interrupts_and_ce().await?;
            Ok(true)
        } else {
            self.device.ce_enable();
            Err(nb::Error::WouldBlock)
        }
    }

    async fn clear_interrupts_and_ce(&mut self) -> nb::Result<(), D::Error> {
        let mut clear = Status(0);
        clear.set_tx_ds(true);
        clear.set_max_rt(true);
        self.device.write_register(clear).await?;

        // Can save power now
        self.device.ce_disable();

        Ok(())
    }

    /// Wait until TX FIFO is empty
    ///
    /// If any packet cannot be delivered and the maximum amount of retries is
    /// reached, the TX FIFO is flushed and all other packets in the FIFO are
    /// lost.
    pub async fn wait_empty(&mut self) -> Result<(), D::Error> {
        let mut empty = false;
        while !empty {
            let (status, fifo_status) = self.device.read_register::<FifoStatus>().await?;
            empty = fifo_status.tx_empty();
            if !empty {
                self.device.ce_enable();
            }

            // TX won't continue while MAX_RT is set
            if status.max_rt() {
                let mut clear = Status(0);
                // If MAX_RT is set, the packet is not removed from the FIFO, so if we do not flush
                // the FIFO, we end up in an infinite loop
                self.device.send_command(&FlushTx).await?;
                // Clear TX interrupts
                clear.set_tx_ds(true);
                clear.set_max_rt(true);
                self.device.write_register(clear).await?;
            }
        }
        // Can save power now
        self.device.ce_disable();

        Ok(())
    }

    /// Read the `OBSERVE_TX` register
    pub async fn observe(&mut self) -> Result<ObserveTx, D::Error> {
        let (_, observe_tx) = self.device.read_register().await?;
        Ok(observe_tx)
    }
}

impl<D: Device> Configuration for TxMode<D> {
    type Inner = D;
    fn device(&mut self) -> &mut Self::Inner {
        &mut self.device
    }
}

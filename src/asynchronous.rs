use crate::{
    cmd::{self, ReadCommand, WriteCommand, WriteDataCommand},
    msg::{self, Decode, Encode},
    Error, Mode, ParticulateMode, I2C_ADDR,
};
use embedded_hal_async::{delay::DelayNs, i2c::I2c};

pub struct AsyncSen5x<I> {
    i2c: I,
    mode: Mode,
    particulates: ParticulateMode,
    addr: u8,
}

impl<I> AsyncSen5x<I> {
    pub const fn new(i2c: I) -> Self {
        Self {
            i2c,
            mode: Mode::Idle,
            particulates: ParticulateMode::Enabled,
            addr: I2C_ADDR,
        }
    }

    /// Set the I²C address of the sensor.
    ///
    /// The [`new()`](Self::new) constructor will use the sensor's default I²C
    /// address (`0x69`). Use this method to set a different address, such as in
    /// cases  an I²C multiplexer is in use.
    #[inline]
    #[must_use]
    pub const fn with_i2c_address(mut self, addr: u8) -> Self {
        self.addr = addr;
        self
    }
}

impl<I> AsyncSen5x<I>
where
    I: I2c,
{
    async fn read_command<C>(&mut self, delay: &mut impl DelayNs) -> Result<C::Rsp, Error<I::Error>>
    where
        C: WriteCommand + ReadCommand,
    {
        self.write_command::<C>(delay).await?;
        let mut buf = C::RSP_BUF;
        self.i2c
            .read(self.addr, buf.as_mut())
            .await
            .map_err(Error::I2cRead)?;
        C::Rsp::decode(&buf).map_err(Error::Decode)
    }

    async fn write_command<C>(&mut self, delay: &mut impl DelayNs) -> Result<(), Error<I::Error>>
    where
        C: WriteCommand,
    {
        self.i2c
            .write(self.addr, &C::COMMAND)
            .await
            .map_err(Error::I2cWrite)?;
        delay.delay_ms(C::EXECUTION_MS as u32).await;
        Ok(())
    }

    async fn write_data_command<C>(
        &mut self,
        delay: &mut impl DelayNs,
        data: C::Data,
    ) -> Result<(), Error<I::Error>>
    where
        C: WriteDataCommand,
    {
        let mut buf = C::REQ_BUF;
        {
            let buf = buf.as_mut();
            buf[..2].copy_from_slice(&C::COMMAND);
            data.encode(&mut buf.as_mut()[2..]);
        };
        self.i2c
            .write(self.addr, &buf.as_ref())
            .await
            .map_err(Error::I2cWrite)?;
        delay.delay_ms(C::EXECUTION_MS as u32).await;
        Ok(())
    }

    pub async fn data_ready(&mut self, delay: &mut impl DelayNs) -> Result<bool, Error<I::Error>> {
        self.read_command::<cmd::ReadDataReady>(delay)
            .await
            .map(|msg::DataReady(ready)| ready)
    }

    pub async fn start_measurement(
        &mut self,
        particulates: ParticulateMode,
        delay: &mut impl DelayNs,
    ) -> Result<(), Error<I::Error>> {
        match particulates {
            ParticulateMode::Enabled => {
                self.write_command::<cmd::StartMeasurement>(delay).await?;
            }
            ParticulateMode::Disabled => {
                self.write_command::<cmd::StartMeasurementNoParticulates>(delay)
                    .await?;
            }
        }
        self.mode = Mode::Measuring;
        self.particulates = particulates;
        Ok(())
    }

    pub async fn stop_measurement(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<(), Error<I::Error>> {
        self.write_command::<cmd::StopMeasurement>(delay).await?;
        self.mode = Mode::Idle;
        Ok(())
    }

    pub async fn read_warm_start_parameter(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<u16, Error<I::Error>> {
        self.read_command::<cmd::WarmStartParameter>(delay).await
    }

    pub async fn set_warm_start_parameter(
        &mut self,
        delay: &mut impl DelayNs,
        param: u16,
    ) -> Result<(), Error<I::Error>> {
        self.mode.check(Mode::Idle)?;
        self.write_data_command::<cmd::WarmStartParameter>(delay, param)
            .await
    }

    pub async fn reset(&mut self, delay: &mut impl DelayNs) -> Result<(), Error<I::Error>> {
        self.write_command::<cmd::Reset>(delay).await?;
        self.mode = Mode::Idle;
        Ok(())
    }

    pub async fn wait_for_data(&mut self, delay: &mut impl DelayNs) -> Result<(), Error<I::Error>> {
        self.wait_for_data_with_interval(delay, 20).await
    }

    // TODO(eliza): consider making this public?
    async fn wait_for_data_with_interval(
        &mut self,
        delay: &mut impl DelayNs,
        interval_ms: u32,
    ) -> Result<(), Error<I::Error>> {
        self.mode.check(Mode::Measuring)?;
        while !self.data_ready(delay).await? {
            delay.delay_ms(interval_ms).await;
        }
        Ok(())
    }

    /// Waits until a measurement is ready and reads data from the sensor.
    pub async fn measure(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<msg::Measurements, Error<I::Error>> {
        self.wait_for_data(delay).await?;
        self.read_command::<cmd::ReadMeasurement>(delay).await
    }

    /// Reads the measurement data from the sensor.
    ///
    /// # Notes
    ///
    /// - In order to read a measurement, the sensor must be in measurement
    ///   mode. Use the [`start_measurement()`](Self::start_measurement) method
    ///   to enter measurement mode.
    ///
    /// - This method does *not* wait for new data to be available. It may
    ///   return the same data multiple times. Use the
    ///   [`data_ready()`](Self::data_ready) method to check if new data is available.
    pub async fn read_measurements(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<msg::Measurements, Error<I::Error>> {
        self.mode.check(Mode::Measuring)?;
        self.read_command::<cmd::ReadMeasurement>(delay).await
    }

    /// Reads raw temperature, relative humidity, VOC, and NOx signals from the
    /// sensor.
    ///
    /// # Notes
    ///
    /// - In order to read a measurement, the sensor must be in measurement
    ///   mode. Use the [`start_measurement()`](Self::start_measurement) method
    ///   to enter measurement mode.
    ///
    /// - This method does *not* wait for new data to be available. It may
    ///   return the same data multiple times. Use the
    ///   [`data_ready()`](Self::data_ready) method to check if new data is
    ///   available.
    ///
    /// - Sensirion does not provide a specification for interpreting these
    ///   values. See the [application note on reading raw signals][appnote] for
    ///   details.
    ///
    /// [appnote]: https://sensirion.com/media/documents/2B6FC1F3/649C3D0E/PS_AN_Read_RHT_VOC_and_NOx_RAW_signals_v2_D1.pdf
    pub async fn read_raw_signals(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<msg::RawSignals, Error<I::Error>> {
        self.mode.check(Mode::Measuring)?;
        self.read_command::<cmd::ReadRawSignals>(delay).await
    }

    pub async fn start_fan_cleaning(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<(), Error<I::Error>> {
        self.mode.check(Mode::Measuring)?;
        self.write_command::<cmd::StartFanCleaning>(delay).await
    }

    pub async fn read_product_name(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<msg::RawString, Error<I::Error>> {
        self.read_command::<cmd::ReadProductName>(delay).await
    }
}

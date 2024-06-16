use crate::{
    cmd::{self, ReadCommand, WriteCommand},
    msg::{self, Decode},
    Error, Mode, ParticulateMode, I2C_ADDR,
};
use embedded_hal_async::{delay::DelayNs, i2c::I2c};

pub struct AsyncSen5x<I> {
    i2c: I,
    mode: Mode,
    particulates: ParticulateMode,
}

impl<I> AsyncSen5x<I> {
    pub fn new(i2c: I) -> Self {
        Self {
            i2c,
            mode: Mode::Idle,
            particulates: ParticulateMode::Enabled,
        }
    }
}

impl<I> AsyncSen5x<I>
where
    I: I2c,
{
    async fn read_command<R: ReadCommand>(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<R::Rsp, Error<I::Error>> {
        let mut buf = R::RSP_BUF;
        self.i2c
            .write(I2C_ADDR, &R::COMMAND)
            .await
            .map_err(Error::I2cWrite)?;
        delay.delay_ms(R::EXECUTION_MS as u32).await;
        self.i2c
            .write_read(I2C_ADDR, &R::COMMAND, buf.as_mut())
            .await
            .map_err(Error::I2cRead)?;
        R::Rsp::decode(&buf).map_err(Error::Decode)
    }

    async fn write_command<R: WriteCommand>(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<(), Error<I::Error>> {
        self.i2c
            .write(I2C_ADDR, &R::COMMAND)
            .await
            .map_err(Error::I2cWrite)?;
        delay.delay_ms(R::EXECUTION_MS as u32).await;
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

    /// Reads the raw measurement data from the sensor.
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
    pub async fn read_raw_measurement(
        &mut self,
        delay: &mut impl DelayNs,
    ) -> Result<msg::RawMeasurement, Error<I::Error>> {
        self.mode.check(Mode::Measuring)?;
        self.read_command::<cmd::ReadMeasurement>(delay).await
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

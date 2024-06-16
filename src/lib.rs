#![no_std]

#[cfg(feature = "embedded-hal-async")]
mod asynchronous;
mod cmd;
mod msg;
pub use msg::*;

const I2C_ADDR: u8 = 0x69; // nice!

#[cfg(feature = "embedded-hal-async")]
pub use self::asynchronous::AsyncSen5x;

pub enum Error<E> {
    /// An I<sup>2</sup>C error occurred during a write operation.
    I2cWrite(E),
    /// An I<sup>2</sup>C error occurred during a register read operation.
    I2cRead(E),
    /// A response message could not be decoded.
    Decode(DecodeError),
    /// The requested operation can only be performed when the sensor is in the
    /// provided mode.
    WrongMode(Mode),
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "fmt", derive(Debug))]
#[repr(u8)]
pub enum Mode {
    Idle,
    Measuring,
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "fmt", derive(Debug))]
#[repr(u8)]
pub enum ParticulateMode {
    Enabled,
    Disabled,
}

impl Mode {
    pub(crate) fn check<E>(self, expected: Self) -> Result<(), Error<E>> {
        if self == expected {
            Ok(())
        } else {
            Err(Error::WrongMode(expected))
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "fmt", derive(Debug))]
#[repr(u8)]
#[non_exhaustive]
pub enum SensorKind {
    Sen50,
    Sen54,
    Sen55,
}

impl core::str::FromStr for SensorKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            s if s.eq_ignore_ascii_case("SEN50") => Ok(Self::Sen50),
            s if s.eq_ignore_ascii_case("SEN54") => Ok(Self::Sen54),
            s if s.eq_ignore_ascii_case("SEN55") => Ok(Self::Sen55),
            _ => Err("expected one of \"SEN50\", \"SEN54\", or \"SEN55\""),
        }
    }
}

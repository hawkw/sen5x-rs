#![no_std]

#[cfg(feature = "embedded-hal-async")]
mod asynchronous;
mod cmd;
mod msg;
pub use msg::*;

const I2C_ADDR: u8 = 0x69; // nice!

#[cfg(feature = "embedded-hal-async")]
pub use self::asynchronous::Sen5xAsync;

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

#[derive(Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "fmt", derive(Debug))]
#[repr(u8)]
#[non_exhaustive]
pub enum SensorKind {
    Sen50,
    Sen54,
    Sen55,
}

// === impl Error ===

#[cfg(feature = "fmt")]
impl<E: core::fmt::Display> core::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::I2cRead(e) => write!(f, "I²C read error: {e}"),
            Self::I2cWrite(e) => write!(f, "I²C write error: {e}"),
            Self::Decode(e) => write!(f, "error decoding message: {e}"),
            Self::WrongMode(mode) => write!(
                f,
                "this operation can only be performed when the sensor is in the {mode:?} mode"
            ),
        }
    }
}

// === impl Mode ===

impl Mode {
    pub(crate) fn check<E>(self, expected: Self) -> Result<(), Error<E>> {
        if self == expected {
            Ok(())
        } else {
            Err(Error::WrongMode(expected))
        }
    }
}

// === impl SensorKind ===

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

use sensirion_i2c::crc8;

pub(crate) trait Decode: Sized {
    type Buf;
    fn decode(buf: &Self::Buf) -> Result<Self, DecodeError>;
}

#[cfg_attr(feature = "fmt", derive(Debug))]

pub enum DecodeError {
    Crc,
    Msg(MessageError),
}

#[cfg_attr(feature = "fmt", derive(Debug))]
pub struct MessageError {
    #[cfg(feature = "fmt")]
    msg: &'static str,
    #[cfg(not(feature = "fmt"))]
    _p: (),
}

pub(crate) struct DataReady(pub(crate) bool);

/// A raw string in the device's representation.
pub struct RawString {
    bytes: [u8; Self::LEN],
    len: usize,
}

/// A measurement from the sensor.
///
/// Raw measurements have the following layout on the wire:
///
/// | Bytes  | Type | Scale factor | Description |
/// |:-------|:-----|:-------------|:----------------------------------------|
/// | 0..1   | u16  | 10           | PM1.0                                   |
/// | 2      | CRC8 |              |                                         |
/// | 3..4   | u16  | 10           | PM2.5                                   |
/// | 5      | CRC8 |              |                                         |
/// | 6..7   | u16  | 10           | PM4.0                                   |
/// | 8      | CRC8 |              |                                         |
/// | 9..10  | u16  | 10           | PM10.0                                  |
/// | 11     | CRC8 |              |                                         |
/// | 12..13 | i16  | 100          | Ambient Humidity (%RH)                  |
/// | 14     | CRC8 |              |                                         |
/// | 15..16 | i16  | 200          | Ambient Temperature (Celcius)           |
/// | 17     | CRC8 |              |                                         |
/// | 18..19 | i16  | 10           | VOC Index                               |
/// | 20     | CRC8 |              |                                         |
/// | 21..22 | i16  | 10           | NOx Index                               |
/// | 23     | CRC8 |              |                                         |
pub struct RawMeasurement {
    pm1_0: Option<u16>,
    pm2_5: Option<u16>,
    pm4_0: Option<u16>,
    pm10_0: Option<u16>,
    rh: Option<i16>,
    temp: Option<i16>,
    voc: Option<i16>,
    nox: Option<i16>,
}

bitflags::bitflags! {
    pub struct SensorStatus: u32 {
        /// `FAN`: Fan failure, fan is mechanically blocked or broken.
        ///
        /// - `0`: Fan works as expected.
        /// - `1`: Fan is switched on, but the measured fan speed is 0 RPM.
        ///
        /// The fan is checked once per second in the measurement mode. If 0 RPM
        /// is measured twice in succession, the `FAN` bit is set.
        /// The `FAN`-bit will not be cleared automatically.
        /// A fan failure can occur if the fan is mechanically blocked or broken.
        const FAN_ERROR = 1 << 4;
        /// `LASER`: Laser failure
        const LASER_ERROR = 1 << 5;
        /// `RHT`: Relative humidity/temperature sensor communication error.
        ///
        /// - `0`: Communication is running normally
        /// - `1`: Error in internal communication with the RH/T sensor.
        const RHT_ERROR = 1 << 6;
        /// `GAS SENSOR`: Gas sensor error (VOC and NOx).
        ///
        /// - `0`: Gas sensor is running normally
        /// - `1`: Gas sensor error.
        const GAS_SENSOR_ERROR = 1 << 7;
        /// `FAN`: Fan cleaning active
        ///
        /// - `0`: Fan is running normally
        /// - `1`: Fan is running its automatic cleaning procedure.
        const FAN_CLEANING = 1 << 19;
        /// `SPEED`: Fan speed out of range.
        ///
        /// - `0`: Fan speed is normal
        /// - `1`: Fan speed is too low or too high.
        const FAN_SPEED_WARNING = 1 << 21;

        /// If any error bit is set.
        const ERROR = Self::FAN_ERROR.bits()
            | Self::LASER_ERROR.bits()
            | Self::RHT_ERROR.bits()
            | Self::GAS_SENSOR_ERROR.bits();
    }
}

// === impl DecodeError ===

impl DecodeError {
    #[cfg(feature = "fmt")]
    fn msg(msg: &'static str) -> Self {
        Self::Msg(MessageError { msg })
    }

    #[cfg(not(feature = "fmt"))]
    fn msg(_: &'static str) -> Self {
        Self::Msg(MessageError { _p: () })
    }
}

// === impl DataReady ===

impl Decode for DataReady {
    type Buf = [u8; 3];
    fn decode(data: &Self::Buf) -> Result<Self, DecodeError> {
        if crc8::calculate(&data[1..2]) != data[2] {
            return Err(DecodeError::Crc);
        }

        if data[0] != 0 {
            return Err(DecodeError::msg("data ready packet must start with 0x00"));
        }
        match data[1] {
            0 => Ok(DataReady(false)),
            1 => Ok(DataReady(true)),
            _ => Err(DecodeError::msg(
                "data ready packet must have 0x00 or 0x01 as second byte",
            )),
        }
    }
}

// === impl RawMeasurement ===

impl Decode for RawMeasurement {
    type Buf = [u8; 24];
    fn decode(buf: &Self::Buf) -> Result<Self, DecodeError> {
        macro_rules! word {
            [$idx:expr; $T:ty] => {{
                let bytes = [buf[$idx], buf[$idx + 1]];
                if crc8::calculate(&bytes) != buf[$idx + 2] {
                    return Err(DecodeError::Crc);
                }
                match <$T>::from_be_bytes(bytes) {
                    <$T>::MAX => None,
                    x => Some(x as $T),
                }
            } };
        }
        Ok(Self {
            pm1_0: word![0; u16],
            pm2_5: word![3; u16],
            pm4_0: word![4; u16],
            pm10_0: word![6; u16],
            rh: word![9; i16],
            temp: word![10; i16],
            voc: word![12; i16],
            nox: word![14; i16],
        })
    }
}

macro_rules! scale_float {
    ($field:expr, $scale:expr) => {
        $field.map(|v| v as f32 / $scale)
    };
}

impl RawMeasurement {
    /// Returns the ambient temperature in Celcius as a [`f32`], or [`None`] if
    /// no temperature reading was present.
    #[must_use]
    pub fn temp_c(&self) -> Option<f32> {
        scale_float!(self.temp, 200.0)
    }

    /// Returns the ambient relative humidity percentage (%RH) as a [`f32`], or
    /// [`None`] if no humidity reading was present.
    #[must_use]
    pub fn relative_humidity(&self) -> Option<f32> {
        scale_float!(self.rh, 100.0)
    }

    /// Returns the volatile organic componds (VOC) index as a [`f32`], or
    /// [`None`] if no VOC index reading was present.
    #[must_use]
    pub fn voc_index(&self) -> Option<f32> {
        scale_float!(self.voc, 10.0)
    }

    /// Returns the nitrogen oxide (NOx) index as a [`f32`], or [`None`] if no
    /// NOx index reading was present.
    #[must_use]
    pub fn nox_index(&self) -> Option<f32> {
        scale_float!(self.nox, 10.0)
    }

    /// Returns the concentration of particulate matter under 1.0 micrometers
    /// (PM<sub>1.0</sub>) in micrograms per cubic meter (µg/m³) as a [`f32`],
    /// or [`None`] if no PM<sub>1.0</sub> reading was present.
    ///
    /// A PM<sub>1.0</sub> reading will not be present in a measurement if the
    /// sensor is not configured to measure particlulate matter concentration
    /// (e.g. measurement was started using the
    /// `start_measurement_no_particulates` command), or if there is an error
    /// with the particulate matter sensor.
    #[must_use]
    pub fn pm1_0(&self) -> Option<f32> {
        scale_float!(self.pm1_0, 10.0)
    }

    /// Returns the concentration of particulate matter under 2.5 micrometers
    /// (PM<sub>2.5</sub>) in micrograms per cubic meter (µg/m³) as a [`f32`],
    /// or [`None`] if no PM<sub>1.0</sub> reading was present.
    ///
    /// A PM<sub>2.5</sub> reading will not be present in a measurement if the
    /// sensor is not configured to measure particlulate matter concentration
    /// (e.g. measurement was started using the
    /// `start_measurement_no_particulates` command), or if there is an error
    /// with the particulate matter sensor.
    #[must_use]
    pub fn pm2_5(&self) -> Option<f32> {
        scale_float!(self.pm2_5, 10.0)
    }

    /// Returns the concentration of particulate matter under 4.0 micrometers
    /// (PM<sub>4.0</sub>) in micrograms per cubic meter (µg/m³) as a [`f32`],
    /// or [`None`] if no PM<sub>4.0</sub> reading was present.
    ///
    /// A PM<sub>4.5</sub> reading will not be present in a measurement if the
    /// sensor is not configured to measure particlulate matter concentration
    /// (e.g. measurement was started using the
    /// `start_measurement_no_particulates` command), or if there is an error
    /// with the particulate matter sensor.
    #[must_use]
    pub fn pm4_0(&self) -> Option<f32> {
        scale_float!(self.pm4_0, 10.0)
    }

    /// Returns the concentration of particulate matter under 4.0 micrometers
    /// (PM<sub>4.0</sub>) in micrograms per cubic meter (µg/m³) as a [`f32`],
    /// or [`None`] if no PM<sub>4.0</sub> reading was present.
    ///
    /// A PM<sub>4.5</sub> reading will not be present in a measurement if the
    /// sensor is not configured to measure particlulate matter concentration
    /// (e.g. measurement was started using the
    /// `start_measurement_no_particulates` command), or if there is an error
    /// with the particulate matter sensor.
    #[must_use]
    pub fn pm10_0(&self) -> Option<f32> {
        scale_float!(self.pm10_0, 10.0)
    }
}

// === impl RawString ===
impl RawString {
    const LEN: usize = 32;

    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("<invalid utf-8>")
    }

    fn push_char(&mut self, c: u8) -> Result<bool, DecodeError> {
        if !c.is_ascii() {
            return Err(DecodeError::msg("non-ASCII character in string"));
        }
        if c == b'\0' {
            return Ok(true);
        }
        self.bytes[self.len] = c;
        self.len += 1;
        Ok(false)
    }
}

impl Decode for RawString {
    type Buf = [u8; 47];
    fn decode(buf: &Self::Buf) -> Result<Self, DecodeError> {
        let mut this = Self {
            bytes: [0; Self::LEN],
            len: 0,
        };
        for chunk in buf.chunks(3) {
            if crc8::calculate(&chunk[..2]) != chunk[2] {
                return Err(DecodeError::Crc);
            }

            if this.push_char(chunk[0])? {
                break;
            }
            if this.push_char(chunk[1])? {
                break;
            }
        }

        Ok(this)
    }
}

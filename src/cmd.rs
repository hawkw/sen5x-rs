use crate::msg::{self, Decode};

pub(crate) trait ReadCommand {
    const RSP_BUF: Self::RspBuf;
    type RspBuf: AsMut<[u8]> + AsRef<[u8]>;
    type Rsp: Decode<Buf = Self::RspBuf>;
}

pub(crate) trait WriteCommand {
    const COMMAND: [u8; 2];
    const EXECUTION_MS: usize;
}

macro_rules! define_read_commands {
    ($(struct $name:ident<$rsp:ty>: $cmd:literal, $exec:literal ms, [$bytes:literal];)+) => {
        $(
            pub(crate) struct $name;

            impl WriteCommand for $name {
                const COMMAND: [u8; 2] = u16::to_be_bytes($cmd);
                const EXECUTION_MS: usize = $exec;
            }

            impl ReadCommand for $name {
                const RSP_BUF: [u8; $bytes] = [0; $bytes];
                type RspBuf = [u8; $bytes];
                type Rsp = $rsp;
            }
        )+
    };
}

macro_rules! define_write_commands {
    ($(struct $name:ident: $cmd:literal, $exec:literal ms;)+) => {
        $(
            pub(crate) struct $name;

            impl WriteCommand for $name {
                const COMMAND: [u8; 2] = u16::to_be_bytes($cmd);
                const EXECUTION_MS: usize = $exec;
            }
        )+
    };
}

define_read_commands! {
    struct ReadDataReady<msg::DataReady>: 0x0202, 20 ms, [3];
    struct ReadMeasurement <msg::RawMeasurement>: 0x03C4, 50 ms, [24];
    struct ReadProductName<msg::RawString>: 0x0306, 20 ms, [47];
    struct ReadSerialNumber<msg::RawString>: 0x0306, 20 ms, [47];
}

define_write_commands! {
    struct StartMeasurement: 0x0021, 50 ms;
    struct StartMeasurementNoParticulates: 0x0037, 50 ms;
    struct StopMeasurement: 0x0104, 200 ms;
    struct StartFanCleaning: 0x5607, 20 ms;
}

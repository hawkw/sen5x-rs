use crate::msg::{self, Decode, Encode};

pub(crate) trait ReadCommand {
    const RSP_BUF: Self::RspBuf;
    type RspBuf: AsMut<[u8]> + AsRef<[u8]>;
    type Rsp: Decode<Buf = Self::RspBuf>;
}

pub(crate) trait WriteCommand {
    const COMMAND: [u8; 2];
    const EXECUTION_MS: usize;
}

pub(crate) trait WriteDataCommand: WriteCommand {
    type Data: Encode;
    const REQ_BUF: Self::ReqBuf;
    type ReqBuf: AsMut<[u8]> + AsRef<[u8]>;
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
    struct ReadMeasurement<msg::Measurements>: 0x03C4, 20 ms, [24];
    struct ReadRawSignals<msg::RawSignals>: 0x03D2, 20 ms, [12];
    struct ReadProductName<msg::RawString>: 0xD014, 20 ms, [47];
    struct ReadSerialNumber<msg::RawString>: 0xD033, 20 ms, [47];
    struct WarmStartParameter<u16>: 0x60C6, 20 ms, [3];
}

impl WriteDataCommand for WarmStartParameter {
    type Data = u16;
    const REQ_BUF: Self::ReqBuf = [0; 5];
    type ReqBuf = [u8; 5];
}

define_write_commands! {
    struct StartMeasurement: 0x0021, 50 ms;
    struct StartMeasurementNoParticulates: 0x0037, 50 ms;
    struct StopMeasurement: 0x0104, 200 ms;
    struct StartFanCleaning: 0x5607, 20 ms;
    struct Reset: 0xD304, 100 ms;
}

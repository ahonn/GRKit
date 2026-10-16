pub const NAME: &str = "gr3-2.10";
pub const FIRMWARE_SHA256: &str =
    "d6a3c9c288d784080e7b283222ecdd8752adfeed6faa36dab44f73115354bfd4";
pub const DECODED_SHA256: &str = "53da690797f35ad6dc2c429d8ca1d0de8e333c211ed959816813982cd093b703";
pub const RTOS_BASE: u32 = 0x53000000;
pub const DATA_START: u32 = 0x55000000;
pub const DATA_SOURCE: u32 = 0x5407b540;
pub const DATA_SIZE: u32 = 0x3a080;
pub const ARRAY_TABLE: u32 = 0x55380f6c;
pub const ARRAY_COUNT: u32 = 0x5537fb58;
pub const HEAP_MIN: u32 = 0x55804374;
pub const HEAP_END: u32 = 0x60000000;
pub const TABLES: [u32; 3] = [0x55018d94, 0x55018cf0, 0x55018e4c];
pub const ENGLISH_SLOT: u32 = 0x55002bb0;
pub const PIXEL_BASE_SLOT: u32 = 0x55018734;
pub use grkit_core::runtime::HEADER_REFERENCE;

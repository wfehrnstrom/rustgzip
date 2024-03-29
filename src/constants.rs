pub const DEFAULT_LEVEL: i8 = 6;
pub const PROGRAM_NAME: &str = "rstzip";
pub const MAX_SUFFIX: usize = 30;
pub const DEFAULT_SUFFIX: &str = "gz";

// File mode constants
pub const S_ISUID: u32 = 0o04000;
pub const S_ISGID: u32 = 0o02000;
pub const S_ISVTX: u32 = 0o01000;

// Status codes
pub const OK: i8 = 0;
pub const WARNING: i8 = 2;
pub const ERROR: i8 = 1;

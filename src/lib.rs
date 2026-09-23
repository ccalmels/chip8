pub mod cd4515;
pub mod cpu;
pub mod machine;
pub mod peripheral;
pub mod ticker;
pub mod windowing;

#[derive(PartialEq)]
pub enum Error {
    UnknownOpCode(u16),
    RomTooLarge { size: usize, capacity: usize },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::UnknownOpCode(opcode) => write!(f, "Unknown opcode: {opcode:#06x}"),
            Error::RomTooLarge { size, capacity } => {
                write!(f, "Rom too large: {size} and {capacity} available")
            }
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for Error {}

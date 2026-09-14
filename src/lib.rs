pub mod cpu;
pub mod machine;
pub mod peripheral;
pub mod windowing;

#[derive(PartialEq, Debug)]
pub enum Error {
    UnknownOpCode(u16),
    RomTooLarge { size: usize, capacity: usize },
}

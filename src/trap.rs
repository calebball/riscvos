use crate::{print, println};

#[derive(Debug)]
pub enum TrapCause {
    SoftwareInterrupt,
    TimerInterrupt,
    ExternalInterrupt,
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAddressMisaligned,
    StoreAccessFault,
    UserEnvironmentCall,
    SupervisorEnvironmentCall,
    InstructionPageFault,
    LoadPageFault,
    StorePageFault,

    ReservedInterrupt,
    PlatformInterrupt,

    ReservedException,
    CustomException,
}

impl From<u64> for TrapCause {
    fn from(val: u64) -> TrapCause {
        let interrupt_bit = val & (1 << 63);
        let exception_code = val & ((1 << 63) - 1);

        match (interrupt_bit, exception_code) {
            (1, 1) => TrapCause::SoftwareInterrupt,
            (1, 5) => TrapCause::TimerInterrupt,
            (1, 9) => TrapCause::ExternalInterrupt,
            (1, c) if c < 16 => TrapCause::ReservedInterrupt,
            (1, _) => TrapCause::PlatformInterrupt,

            (0, 0) => TrapCause::InstructionAddressMisaligned,
            (0, 1) => TrapCause::InstructionAccessFault,
            (0, 2) => TrapCause::IllegalInstruction,
            (0, 3) => TrapCause::Breakpoint,
            (0, 4) => TrapCause::LoadAddressMisaligned,
            (0, 5) => TrapCause::LoadAccessFault,
            (0, 6) => TrapCause::StoreAddressMisaligned,
            (0, 7) => TrapCause::StoreAccessFault,
            (0, 8) => TrapCause::UserEnvironmentCall,
            (0, 9) => TrapCause::SupervisorEnvironmentCall,
            (0, 12) => TrapCause::InstructionPageFault,
            (0, 13) => TrapCause::LoadPageFault,
            (0, 15) => TrapCause::StorePageFault,

            (0, c) if c >= 24 && c <= 31 => TrapCause::CustomException,
            (0, c) if c >= 48 && c <= 63 => TrapCause::CustomException,
            (0, _) => TrapCause::ReservedException,

            (_, _) => panic!("Interrupt bit > 1 in when decoding trap cause?")
        }
    }
}

#[no_mangle]
pub extern "C" fn kernel_trap(cause: u64) {
    let cause: TrapCause = cause.into();
    panic!("Unhandled trap: {:?}", cause);
}

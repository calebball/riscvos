use core::arch::global_asm;

global_asm!(include_str!("boot.S"));
global_asm!(include_str!("memory_layout.S"));
global_asm!(include_str!("trap.S"));

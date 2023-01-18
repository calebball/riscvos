#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod asm;
pub mod page_allocator;
pub mod page_table;
pub mod serial;

#[cfg(test)]
pub mod test;

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test::panic_handler(info);

    loop {}
}

#[cfg(test)]
#[no_mangle]
extern "C" fn kernel_main() -> ! {
    test_main();

    loop {}
}

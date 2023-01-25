#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(once_cell)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::cell::OnceCell;
use spin::Mutex;

pub mod asm;
pub mod page_allocator;
pub mod page_table;
pub mod serial;
pub mod trap;

#[cfg(test)]
pub mod test;

use crate::page_allocator::{PageAddr, PageAllocator};
use crate::page_table::VirtualMemory;
use core::arch::asm;

static VIRTUAL_MEMORY: Mutex<OnceCell<VirtualMemory>> = Mutex::new(OnceCell::new());

const PAGE_SIZE: u64 = 4096;

extern "C" {
    static HEAP_START: u64;
    static HEAP_END: u64;
    static TRAP: u64;
}

#[no_mangle]
pub unsafe extern "C" fn initialise_kernel() {
    let page_allocator = PageAllocator::new(
        PageAddr {
            address: HEAP_START + PAGE_SIZE - (HEAP_START % PAGE_SIZE),
        },
        PageAddr {
            address: HEAP_END - (HEAP_END % PAGE_SIZE) - 1,
        },
    );
    let mut vm = VirtualMemory::new(page_allocator).unwrap();
    vm.init().unwrap();
    asm!("csrw satp, {}", in(reg) vm.satp());
    VIRTUAL_MEMORY.lock().set(vm).unwrap();
    asm!("csrw stvec, {}", in(reg) TRAP);
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test::panic_handler(info);

    loop {}
}

#[cfg(test)]
#[no_mangle]
extern "C" fn kernel_main() -> ! {
    println!("ohhai tester");

    test_main();

    loop {}
}

#[cfg(test)]
mod temp_test {
    use super::*;
    use crate::page_table::PageTableEntryMode;

    #[test_case]
    fn read_virtual_address() {
        let virtual_address = 0x90000000;
        VIRTUAL_MEMORY
            .lock()
            .get_mut()
            .map(|vm| {
                vm.map(
                    virtual_address.try_into().unwrap(),
                    PageTableEntryMode::ReadWrite,
                )
                .unwrap();
            })
            .unwrap();
        let ptr = virtual_address as *mut u8;
        unsafe {
            ptr.write(1);
            assert_eq!(ptr.read(), 1);
        }
    }

    #[test_case]
    fn enter_trap() {
        use core::ptr;
        // unsafe { (ptr::null() as *const u64).read(); }
        // unsafe { (0 as *const u64).read(); }
        unsafe { (u64::MAX as *const u8).read(); }
    }
}

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
}

#[no_mangle]
pub unsafe extern "C" fn initialise_kernel() {
    println!("enter init");
    let page_allocator = PageAllocator::new(
        PageAddr {
            address: HEAP_START + PAGE_SIZE - (HEAP_START % PAGE_SIZE),
        },
        PageAddr {
            address: HEAP_END - (HEAP_END % PAGE_SIZE) - 1,
        },
    );
    println!("page allocator created");
    let mut vm = VirtualMemory::new(page_allocator).unwrap();
    println!("virtual memory created");
    vm.init().unwrap();
    println!("virtual memory initialised");
    asm!("csrw satp, {}", in(reg) vm.satp());
    println!("SATP written");
    VIRTUAL_MEMORY.lock().set(vm).unwrap();
    println!(
        "Code is mapped to {:#0x}",
        VIRTUAL_MEMORY
            .lock()
            .get()
            .map(|vm| vm.translate(0x8000003c.try_into().unwrap()).unwrap())
            .unwrap()
            .address
    );
    println!(
        "Root table is at {:#0x}",
        VIRTUAL_MEMORY.lock().get().unwrap().root_table as u64
    );
    unsafe {
        let satp: u64;
        asm!("csrr {}, satp", out(reg) satp);
        println!("SATP is {:#0x}", satp);
    }
    println!("run");
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
}

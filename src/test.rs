use crate::{print, println};

const SIFIVE_TEST_ADDR: u64 = 0x100000;

#[repr(u32)]
enum QemuExitCode {
    Failure = 0x3333,
    Success = 0x5555,
    Reset = 0x7777,
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn panic_handler(info: &core::panic::PanicInfo) {
    println!("[failed]");
    println!("Error: {}", info);
    exit_qemu(QemuExitCode::Failure);
}

fn exit_qemu(exit_code: QemuExitCode) {
    let ptr: *mut u32 = SIFIVE_TEST_ADDR as *mut u32;

    println!("exiting...");

    unsafe {
        ptr.write_volatile(exit_code as u32);
    }
}

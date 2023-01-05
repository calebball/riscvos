use core::fmt;

use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::MmioSerialPort;

const QEMU_UART0_ADDRESS: u64 = 0x1000_0000;

lazy_static! {
    pub static ref QEMU_SERIAL: Mutex<MmioSerialPort> = {
        let mut port = unsafe { MmioSerialPort::new(QEMU_UART0_ADDRESS as usize) };
        port.init();
        Mutex::new(port)
    };
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    QEMU_SERIAL.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

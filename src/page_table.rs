use crate::page_allocator::{PageAddr, PageAllocationError, PageAllocator, PageRange};
use core::ptr;

extern "C" {
    static TEXT_START: u64;
    static TEXT_END: u64;
    static RODATA_START: u64;
    static RODATA_END: u64;
    static DATA_START: u64;
    static DATA_END: u64;
    static BSS_START: u64;
    static BSS_END: u64;
    static STACK_START: u64;
    static STACK_END: u64;
    static HEAP_START: u64;
    static HEAP_END: u64;
}

#[derive(Debug)]
pub struct PhysicalAddress {
    pub address: u64,
}

impl PhysicalAddress {
    pub fn new(address: u64) -> Self {
        Self { address }
    }
}

impl From<u64> for PhysicalAddress {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

#[derive(Debug)]
pub enum VirtualAddressError {
    OutOfVirtualMemoryRange,
}

#[derive(Debug, Clone)]
pub struct VirtualAddress {
    value: u64,
}

impl VirtualAddress {
    pub fn page_table_index(&self, level: u64) -> u64 {
        let mask = (1 << 9) - 1;
        (self.value >> 12 + level * 9) & mask
    }

    pub fn offset(&self) -> u64 {
        let mask = !((1 << 12) - 1);
        self.value & mask
    }
}

impl TryFrom<u64> for VirtualAddress {
    type Error = VirtualAddressError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let mask = !((1 << 39) - 1);
        if value & mask != 0 {
            return Err(VirtualAddressError::OutOfVirtualMemoryRange);
        }

        match (value >> 38) & 1 {
            1 => Ok(VirtualAddress {
                value: value | mask,
            }),
            0 => Ok(VirtualAddress {
                value: value & (!mask),
            }),
            e => panic!("Bitwise and with 1 returned {}", e),
        }
    }
}

impl TryFrom<PageAddr> for VirtualAddress {
    type Error = VirtualAddressError;

    fn try_from(value: PageAddr) -> Result<Self, Self::Error> {
        value.address.try_into()
    }
}

#[derive(Debug)]
pub enum PageTableEntryMode {
    PageTablePointer,
    ReadOnly,
    ReadWrite,
    ExecuteOnly,
    ReadExecute,
    ReadWriteExecute,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PageTableEntry {
    value: u64,
}

impl PageTableEntry {
    pub fn is_valid(&self) -> bool {
        self.value & (1 << 0) == 1
    }

    pub fn is_readable(&self) -> bool {
        self.value & (1 << 1) == 1
    }

    pub fn is_writable(&self) -> bool {
        self.value & (1 << 2) == 1
    }

    pub fn is_executable(&self) -> bool {
        self.value & (1 << 3) == 1
    }

    pub fn is_leaf(&self) -> bool {
        self.value & (0b1110) > 0
    }

    pub fn is_user_accessible(&self) -> bool {
        self.value & (1 << 4) == 1
    }

    pub fn is_global(&self) -> bool {
        self.value & (1 << 5) == 1
    }

    pub fn has_been_accessed(&self) -> bool {
        self.value & (1 << 6) == 1
    }

    pub fn is_dirty(&self) -> bool {
        self.value & (1 << 7) == 1
    }

    pub fn physical_page(&self) -> u64 {
        (self.value >> 10) & ((1 << 44) - 1)
    }

    pub fn physical_page_number_0(&self) -> u64 {
        (self.value >> 10) & 0b1_1111_1111
    }

    pub fn physical_page_number_1(&self) -> u64 {
        (self.value >> 19) & 0b1_1111_1111
    }

    pub fn physical_page_number_2(&self) -> u64 {
        (self.value >> 28) & ((1 << 26) - 1)
    }
}

impl From<PageTableEntryBuilder> for PageTableEntry {
    fn from(b: PageTableEntryBuilder) -> Self {
        if let Some(value) = b.invalid {
            return Self { value };
        }

        let mut value = 1;

        match b.mode {
            PageTableEntryMode::PageTablePointer => (),
            PageTableEntryMode::ReadOnly => value |= 1 << 1,
            PageTableEntryMode::ReadWrite => value |= (1 << 1) + (1 << 2),
            PageTableEntryMode::ExecuteOnly => value |= 1 << 3,
            PageTableEntryMode::ReadExecute => value |= (1 << 1) + (1 << 3),
            PageTableEntryMode::ReadWriteExecute => value |= (1 << 1) + (1 << 2) + (1 << 3),
        }

        if b.user {
            value |= 1 << 4;
        }

        if b.global {
            value |= 1 << 5;
        }

        value |= (b.page_number >> 12) << 10;

        PageTableEntry { value }
    }
}

struct PageTableEntryBuilder {
    mode: PageTableEntryMode,
    user: bool,
    global: bool,
    page_number: u64,
    invalid: Option<u64>,
}

impl PageTableEntryBuilder {
    pub fn new(page_number: u64, mode: PageTableEntryMode) -> Self {
        Self {
            mode,
            user: false,
            global: false,
            page_number,
            invalid: None,
        }
    }

    pub fn user_accessible(mut self) -> Self {
        self.user = true;
        self
    }

    pub fn global_mapping(mut self) -> Self {
        self.global = true;
        self
    }

    pub fn invalid(value: u64) -> Self {
        Self {
            mode: PageTableEntryMode::PageTablePointer,
            user: false,
            global: false,
            page_number: 0,
            invalid: Some(value & (u64::MAX - 1)),
        }
    }

    pub fn build(self) -> PageTableEntry {
        self.into()
    }
}

pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn new(allocator: &mut PageAllocator) -> Result<*mut Self, PageAllocationError> {
        let page = allocator.alloc()?.address as *mut Self;
        unsafe {
            *page = Self {
                entries: [PageTableEntryBuilder::invalid(0).build(); 512],
            };
        }
        Ok(page)
    }

    pub fn walk(&mut self, virt: VirtualAddress) -> Option<*mut PageTableEntry> {
        self.do_walk(virt, 2)
    }

    fn do_walk(&mut self, virt: VirtualAddress, level: u64) -> Option<*mut PageTableEntry> {
        let pte_idx = virt.page_table_index(level) as usize;
        let pte = self.entries[pte_idx];
        let pte_ptr =
            unsafe { (ptr::addr_of_mut!(self.entries) as *mut PageTableEntry).add(pte_idx) };

        if level == 0 {
            return Some(pte_ptr);
        }

        if !pte.is_valid() {
            return None;
        }

        let next: &mut PageTable = unsafe {
            ((pte.physical_page() << 12) as *mut PageTable)
                .as_mut()
                .unwrap()
        };
        next.do_walk(virt, level - 1)
    }

    pub fn walk_and_map(
        &mut self,
        virt: VirtualAddress,
        allocator: &mut PageAllocator,
    ) -> Result<*mut PageTableEntry, PageAllocationError> {
        self.do_walk_and_map(virt, 2, allocator)
    }

    fn do_walk_and_map(
        &mut self,
        virt: VirtualAddress,
        level: u64,
        allocator: &mut PageAllocator,
    ) -> Result<*mut PageTableEntry, PageAllocationError> {
        // println!(
        //     "In table {:#0x} at level {}",
        //     (self as *mut PageTable) as u64,
        //     level
        // );
        let pte_idx = virt.page_table_index(level) as usize;
        let pte = self.entries[pte_idx];
        let pte_ptr =
            unsafe { (ptr::addr_of_mut!(self.entries) as *mut PageTableEntry).add(pte_idx) };

        // println!("  Getting entry {}: {:#064b}", pte_idx, pte.value);

        if level == 0 {
            // println!("  Entry at {:#0x}", ptr::addr_of_mut!(pte) as u64);
            // println!(
            //     "  Or maybe it should be {:#0x}",
            //     ptr::addr_of!(self.entries) as u64 + (pte_idx * 8) as u64
            // );
            // println!("  Or even {:#0x}", pte_ptr as u64);
            return Ok(pte_ptr);
        }

        let next: &mut PageTable = if !pte.is_valid() {
            let new_page = allocator.alloc()?;
            unsafe {
                pte_ptr.write(
                    PageTableEntryBuilder::new(
                        new_page.address,
                        PageTableEntryMode::PageTablePointer,
                    )
                    .build(),
                );
            }
            unsafe { (new_page.address as *mut PageTable).as_mut().unwrap() }
        } else {
            unsafe {
                ((pte.physical_page() << 12) as *mut PageTable)
                    .as_mut()
                    .unwrap()
            }
        };

        next.do_walk_and_map(virt, level - 1, allocator)
    }
}

#[derive(Debug)]
pub struct VirtualMemory {
    pub page_allocator: PageAllocator,
    pub root_table: *mut PageTable,
}

unsafe impl Send for VirtualMemory {}

impl VirtualMemory {
    pub fn new(mut page_allocator: PageAllocator) -> Result<Self, PageAllocationError> {
        let root_table = PageTable::new(&mut page_allocator)?;

        Ok(Self {
            page_allocator,
            root_table,
        })
    }

    unsafe fn map_to(
        &mut self,
        virt: VirtualAddress,
        phys: PageAddr,
        mode: PageTableEntryMode,
    ) -> Result<(), PageAllocationError> {
        let pte = (*self.root_table).walk_and_map(virt, &mut self.page_allocator)?;
        pte.write(PageTableEntryBuilder::new(phys.address, mode).build());
        Ok(())
    }

    pub fn map(
        &mut self,
        virt: VirtualAddress,
        mode: PageTableEntryMode,
    ) -> Result<(), PageAllocationError> {
        let phys = self.page_allocator.alloc()?;
        unsafe { self.map_to(virt, phys, mode) }
    }

    pub fn identity_map(
        &mut self,
        phys: PageAddr,
        mode: PageTableEntryMode,
    ) -> Result<(), PageAllocationError> {
        unsafe { self.map_to(phys.clone().try_into().unwrap(), phys, mode) }
    }

    pub fn translate(&self, virt: VirtualAddress) -> Option<PhysicalAddress> {
        let pte = unsafe { *(*self.root_table).walk(virt.clone())? };
        if !(pte.is_leaf() && pte.is_valid()) {
            return None;
        }
        Some(((pte.physical_page() << 12) | virt.offset()).into())
    }

    pub fn init(&mut self) -> Result<(), PageAllocationError> {
        unsafe {
            for page in PageRange::new(
                PageAddr {
                    address: TEXT_START,
                },
                PageAddr { address: TEXT_END },
            ) {
                self.identity_map(page, PageTableEntryMode::ReadExecute)?
            }

            for page in PageRange::new(
                PageAddr {
                    address: RODATA_START,
                },
                PageAddr {
                    address: RODATA_END,
                },
            ) {
                self.identity_map(page, PageTableEntryMode::ReadOnly)?
            }

            for page in PageRange::new(
                PageAddr {
                    address: DATA_START,
                },
                PageAddr { address: DATA_END },
            ) {
                self.identity_map(page, PageTableEntryMode::ReadWrite)?
            }

            for page in PageRange::new(
                PageAddr { address: BSS_START },
                PageAddr { address: BSS_END },
            ) {
                self.identity_map(page, PageTableEntryMode::ReadWrite)?
            }

            for page in PageRange::new(
                PageAddr {
                    address: STACK_START,
                },
                PageAddr { address: STACK_END },
            ) {
                self.identity_map(page, PageTableEntryMode::ReadWrite)?
            }

            for page in PageRange::new(
                PageAddr {
                    address: HEAP_START,
                },
                PageAddr { address: HEAP_END },
            ) {
                self.identity_map(page, PageTableEntryMode::ReadWrite)?
            }

            self.identity_map(
                PageAddr {
                    address: 0x1000_0000,
                },
                PageTableEntryMode::ReadWrite,
            )?;

            self.identity_map(
                PageAddr {
                    address: 0x0010_0000,
                },
                PageTableEntryMode::ReadWrite,
            )?;
        }
        Ok(())
    }

    pub fn satp(&self) -> u64 {
        let addr = self.root_table as u64;
        (8 << 60) | (addr >> 12)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::page_allocator::test::test_page_allocator;

    #[test_case]
    fn creating_a_new_page_table_reduces_free_page_count_by_1() {
        let mut allocator = test_page_allocator(10);
        let _ = PageTable::new(&mut allocator);
        assert_eq!(allocator.free_pages(), 9);
    }

    #[test_case]
    fn walking_a_fresh_table_returns_none() {
        let mut allocator = test_page_allocator(10);
        let table = unsafe { PageTable::new(&mut allocator).unwrap().as_mut().unwrap() };
        assert_eq!(table.walk(0.try_into().unwrap()), None)
    }

    #[test_case]
    fn walking_and_mapping_a_fresh_table_returns_some() {
        let mut allocator = test_page_allocator(10);
        let table = unsafe { &mut *PageTable::new(&mut allocator).unwrap() };
        assert!(table
            .walk_and_map(0.try_into().unwrap(), &mut allocator)
            .is_ok())
    }

    #[test_case]
    fn walking_and_mapping_a_fresh_table_reduces_page_count() {
        let mut allocator = test_page_allocator(10);
        let table = unsafe { &mut *PageTable::new(&mut allocator).unwrap() };
        table
            .walk_and_map(0.try_into().unwrap(), &mut allocator)
            .unwrap();
        assert_eq!(allocator.free_pages(), 7);
    }

    #[test_case]
    fn walk_and_map_followed_by_walk_agrees() {
        let mut allocator = test_page_allocator(128);
        let table = unsafe { &mut *PageTable::new(&mut allocator).unwrap() };
        let target_addr = 0x80001000;
        let first_walk = table
            .walk_and_map(target_addr.try_into().unwrap(), &mut allocator)
            .unwrap() as u64;
        let second_walk = table.walk(target_addr.try_into().unwrap()).unwrap() as u64;
        assert_eq!(first_walk, second_walk);
    }

    #[test_case]
    fn initialising_virtual_memory_succeeds() {
        let allocator = test_page_allocator(128);
        let mut vm = VirtualMemory::new(allocator).unwrap();
        assert!(vm.init().is_ok());
    }
}

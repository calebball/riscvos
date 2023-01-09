use lazy_static::lazy_static;
use spin::Mutex;

const PAGE_SIZE: u64 = 4096;

extern "C" {
    pub static HEAP_START: u64;
    pub static HEAP_END: u64;
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct PageAddr {
    address: u64,
}

impl PageAddr {
    fn as_mut_ptr(self) -> *mut u8 {
        return self.address as *mut u8;
    }
}

struct PageRange {
    next_page: u64,
    last_page: u64,
}

impl PageRange {
    fn new(start: PageAddr, end: PageAddr) -> Self {
        Self {
            next_page: start.address,
            last_page: end.address,
        }
    }
}

impl Iterator for PageRange {
    type Item = PageAddr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_page > self.last_page {
            return None;
        }

        let address = self.next_page;
        self.next_page += PAGE_SIZE;

        Some(PageAddr { address })
    }
}

struct FreePageNode {
    next: Option<*mut FreePageNode>,
}

#[derive(Debug)]
pub enum PageAllocationError {
    NoPagesAvailable,
}

pub struct PageAllocator {
    free_list: Option<*mut FreePageNode>,
}

impl PageAllocator {
    pub unsafe fn new(heap_start: PageAddr, heap_end: PageAddr) -> Self {
        let mut result = Self { free_list: None };

        for page in PageRange::new(heap_start, heap_end) {
            result.dealloc(page);
        }

        result
    }

    pub fn alloc(&mut self) -> Result<PageAddr, PageAllocationError> {
        match self.free_list {
            None => return Err(PageAllocationError::NoPagesAvailable),
            Some(page_ptr) => {
                let page_address = PageAddr {
                    address: page_ptr as u64,
                };

                unsafe {
                    self.free_list = (*page_ptr).next;
                }

                unsafe {
                    for i in 0..PAGE_SIZE {
                        let byte_ptr = page_ptr.add(i as usize) as *mut u8;
                        *byte_ptr = 0;
                    }
                }

                Ok(page_address)
            }
        }
    }

    pub fn dealloc(&mut self, page: PageAddr) {
        let next_node = FreePageNode {
            next: self.free_list,
        };

        let page_ptr = page.as_mut_ptr() as *mut FreePageNode;

        unsafe {
            *page_ptr = next_node;
        }

        self.free_list = Some(page_ptr)
    }
}

unsafe impl Send for PageAllocator {}

lazy_static! {
    pub static ref PAGE_ALLOCATOR: Mutex<PageAllocator> = {
        Mutex::new(unsafe {
            PageAllocator::new(
                PageAddr {
                    address: HEAP_START + PAGE_SIZE - (HEAP_START % PAGE_SIZE),
                },
                PageAddr { address: HEAP_END },
            )
        })
    };
}

#[cfg(test)]
mod test {
    use super::*;

    fn heap_addresses(size: u64) -> (PageAddr, PageAddr) {
        let heap_start_address = unsafe { HEAP_START + PAGE_SIZE - (HEAP_START % PAGE_SIZE) };
        let heap_start = PageAddr {
            address: heap_start_address,
        };
        let heap_end = PageAddr {
            address: heap_start_address + (size - 1) * PAGE_SIZE,
        };
        (heap_start, heap_end)
    }

    #[test_case]
    fn initialising_an_allocator_succeeds() {
        let (heap_start, heap_end) = heap_addresses(1);

        let allocator = unsafe { PageAllocator::new(heap_start, heap_end) };

        assert!(allocator.free_list.is_some());
    }

    #[test_case]
    fn allocating_one_page_succeeds() {
        let (heap_start, heap_end) = heap_addresses(1);
        let expected = heap_start.address;

        let mut allocator = unsafe { PageAllocator::new(heap_start, heap_end) };

        let page = allocator.alloc();
        assert!(page.is_ok());
        assert_eq!(page.unwrap().address, expected);
    }

    #[test_case]
    fn allocating_two_pages_succeeds() {
        let (heap_start, heap_end) = heap_addresses(2);
        let first_expected = heap_start.address + 1 * PAGE_SIZE;
        let second_expected = heap_start.address + 0 * PAGE_SIZE;

        let mut allocator = unsafe { PageAllocator::new(heap_start, heap_end) };

        let page_one = allocator.alloc();
        assert!(page_one.is_ok());
        assert_eq!(page_one.unwrap().address, first_expected);

        let page_two = allocator.alloc();
        assert!(page_two.is_ok());
        assert_eq!(page_two.unwrap().address, second_expected);
    }

    #[test_case]
    fn allocating_too_many_pages_fails() {
        let (heap_start, heap_end) = heap_addresses(1);

        let mut allocator = unsafe { PageAllocator::new(heap_start, heap_end) };

        let _ = allocator.alloc();
        let page_two = allocator.alloc();

        assert!(page_two.is_err());
    }

    #[test_case]
    fn deallocating_a_page_succeeds() {
        let (heap_start, heap_end) = heap_addresses(1);

        let mut allocator = unsafe { PageAllocator::new(heap_start, heap_end) };

        let page = allocator.alloc();
        allocator.dealloc(page.unwrap());
    }

    #[test_case]
    fn deallocating_a_page_keeps_other_page_allocated() {
        let (heap_start, heap_end) = heap_addresses(2);

        let mut allocator = unsafe { PageAllocator::new(heap_start, heap_end) };

        let page_one = allocator.alloc();
        let _ = allocator.alloc();

        allocator.dealloc(page_one.unwrap());

        assert!(allocator.free_list.is_some());
        assert!(unsafe { (*allocator.free_list.unwrap()).next.is_none() });
    }

    #[test_case]
    fn deallocating_two_pages_succeeds() {
        let (heap_start, heap_end) = heap_addresses(2);

        let mut allocator = unsafe { PageAllocator::new(heap_start, heap_end) };

        let page_one = allocator.alloc();
        let page_two = allocator.alloc();

        allocator.dealloc(page_two.unwrap());
        allocator.dealloc(page_one.unwrap());
    }
}

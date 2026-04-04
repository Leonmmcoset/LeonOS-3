use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const HEAP_SIZE: usize = 1024 * 1024;

struct BumpAllocator {
    heap_start: AtomicUsize,
    heap_end: AtomicUsize,
    next: AtomicUsize,
    initialized: AtomicBool,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            heap_start: AtomicUsize::new(0),
            heap_end: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
            initialized: AtomicBool::new(false),
        }
    }

    fn init(&self, heap_start: usize, heap_size: usize) {
        self.heap_start.store(heap_start, Ordering::SeqCst);
        self.heap_end.store(heap_start + heap_size, Ordering::SeqCst);
        self.next.store(heap_start, Ordering::SeqCst);
        self.initialized.store(true, Ordering::SeqCst);
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !self.initialized.load(Ordering::SeqCst) {
            return null_mut();
        }

        let align = layout.align();
        let size = layout.size();
        let heap_end = self.heap_end.load(Ordering::SeqCst);

        let mut current = self.next.load(Ordering::SeqCst);
        loop {
            let alloc_start = align_up(current, align);
            let Some(alloc_end) = alloc_start.checked_add(size) else {
                return null_mut();
            };

            if alloc_end > heap_end {
                return null_mut();
            }

            match self.next.compare_exchange(
                current,
                alloc_end,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return alloc_start as *mut u8,
                Err(next) => current = next,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator does not reclaim memory.
    }
}

const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

pub fn init_heap() {
    if ALLOCATOR.initialized.load(Ordering::SeqCst) {
        return;
    }

    unsafe {
        let heap_start = HEAP.as_ptr() as usize;
        ALLOCATOR.init(heap_start, HEAP_SIZE);
    }
}

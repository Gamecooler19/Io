use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::ptr::NonNull;

#[global_allocator]
static ALLOCATOR: PoolAllocator = PoolAllocator::new();

pub struct PoolAllocator {
    small_pools: [Pool; 32],
    large_alloc: System,
}

impl PoolAllocator {
    const fn new() -> Self {
        Self {
            small_pools: [Pool::new(); 32],
            large_alloc: System,
        }
    }
}

unsafe impl GlobalAlloc for PoolAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() <= 512 {
            let index = (layout.size() - 1) / 16;
            self.small_pools[index].alloc(layout)
        } else {
            self.large_alloc.alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() <= 512 {
            let index = (layout.size() - 1) / 16;
            self.small_pools[index].dealloc(ptr, layout)
        } else {
            self.large_alloc.dealloc(ptr, layout)
        }
    }
}

struct Pool {
    blocks: Cell<Vec<NonNull<u8>>>,
    block_size: usize,
}

impl Pool {
    const fn new() -> Self {
        Self {
            blocks: Cell::new(Vec::new()),
            block_size: 4096,
        }
    }

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut blocks = self.blocks.take();
        
        // Try to reuse an existing block
        if let Some(block) = blocks.pop() {
            self.blocks.set(blocks);
            return block.as_ptr();
        }
        
        // Allocate a new block
        let size = std::cmp::max(layout.size(), self.block_size);
        let align = layout.align();
        
        // Create new memory block
        match Self::allocate_block(size, align) {
            Some(ptr) => {
                // Initialize block metadata
                let meta = BlockMetadata {
                    size,
                    align,
                    in_use: true,
                };
                
                // Store metadata at the start of the block
                ptr.cast::<BlockMetadata>().write(meta);
                
                // Return pointer to usable memory (after metadata)
                ptr.add(std::mem::size_of::<BlockMetadata>())
            }
            None => std::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Get block metadata
        let meta_ptr = ptr.sub(std::mem::size_of::<BlockMetadata>())
            .cast::<BlockMetadata>();
        let meta = meta_ptr.read();
        
        if !meta.in_use {
            // Double free detected
            std::panic!("Double free detected");
        }
        
        // Mark block as free
        meta_ptr.write(BlockMetadata {
            size: meta.size,
            align: meta.align,
            in_use: false,
        });
        
        // Add to free blocks list
        let mut blocks = self.blocks.take();
        blocks.push(NonNull::new_unchecked(ptr));
        self.blocks.set(blocks);
        
        // Coalesce adjacent free blocks if possible
        self.coalesce_blocks();
    }

    fn allocate_block(size: usize, align: usize) -> Option<*mut u8> {
        let layout = Layout::from_size_align(
            size + std::mem::size_of::<BlockMetadata>(),
            align
        ).ok()?;
        
        let ptr = unsafe { System.alloc(layout) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr)
        }
    }

    fn coalesce_blocks(&self) {
        let mut blocks = self.blocks.take();
        blocks.sort_by_key(|b| b.as_ptr() as usize);
        
        let mut i = 0;
        while i < blocks.len().saturating_sub(1) {
            let current = blocks[i].as_ptr();
            let next = blocks[i + 1].as_ptr();
            
            unsafe {
                let current_meta = current.cast::<BlockMetadata>().read();
                let next_meta = next.cast::<BlockMetadata>().read();
                
                if current.add(current_meta.size) == next {
                    // Merge blocks
                    let merged_size = current_meta.size + next_meta.size;
                    current.cast::<BlockMetadata>().write(BlockMetadata {
                        size: merged_size,
                        align: current_meta.align,
                        in_use: false,
                    });
                    
                    blocks.remove(i + 1);
                    continue;
                }
            }
            i += 1;
        }
        
        self.blocks.set(blocks);
    }
}

#[derive(Debug, Clone, Copy)]
struct BlockMetadata {
    size: usize,
    align: usize,
    in_use: bool,
}

pub struct MemoryManager;

impl MemoryManager {
    pub fn new() -> Self {
        Self
    }

    pub fn initialize(&self) {
        // Initialize memory tracking
        let mut stats = MemoryStats::default();
        
        // Configure memory pools
        for (i, pool) in ALLOCATOR.small_pools.iter().enumerate() {
            let block_size = (i + 1) * 16;
            stats.register_pool(block_size);
            
            // Pre-allocate some blocks for common sizes
            if block_size <= 64 {
                unsafe {
                    for _ in 0..4 {
                        let layout = Layout::from_size_align(block_size, 8).unwrap();
                        let ptr = pool.alloc(layout);
                        if !ptr.is_null() {
                            stats.track_allocation(block_size);
                        }
                    }
                }
            }
        }
        
        // Initialize memory limits
        self.set_memory_limits();
        
        // Register OOM handler
        std::alloc::set_alloc_error_hook(|layout| {
            eprintln!("Out of memory: failed to allocate {} bytes", layout.size());
            stats.dump_memory_stats();
            std::process::abort();
        });
    }

    fn set_memory_limits(&self) {
        // Get system memory info
        #[cfg(target_os = "linux")]
        {
            use std::fs::File;
            use std::io::Read;
            
            if let Ok(mut file) = File::open("/proc/meminfo") {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    if let Some(mem_total) = parse_meminfo(&contents) {
                        // Set limits to 80% of available memory
                        let limit = (mem_total as f64 * 0.8) as usize;
                        MEMORY_LIMIT.store(limit, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }
        }
    }
}

#[derive(Default)]
struct MemoryStats {
    allocations: std::sync::atomic::AtomicUsize,
    total_bytes: std::sync::atomic::AtomicUsize,
    pool_sizes: Vec<usize>,
}

impl MemoryStats {
    fn register_pool(&mut self, size: usize) {
        self.pool_sizes.push(size);
    }
    
    fn track_allocation(&self, size: usize) {
        self.allocations.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.total_bytes.fetch_add(size, std::sync::atomic::Ordering::SeqCst);
    }
    
    fn dump_memory_stats(&self) {
        eprintln!("Memory Statistics:");
        eprintln!("Total allocations: {}", self.allocations.load(std::sync::atomic::Ordering::SeqCst));
        eprintln!("Total bytes allocated: {}", self.total_bytes.load(std::sync::atomic::Ordering::SeqCst));
        eprintln!("Pool sizes: {:?}", self.pool_sizes);
    }
}

static MEMORY_LIMIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

#[cfg(target_os = "linux")]
fn parse_meminfo(contents: &str) -> Option<usize> {
    for line in contents.lines() {
        if line.starts_with("MemTotal:") {
            return line.split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
                .map(|kb| kb * 1024);
        }
    }
    None
}

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const CLOCK_FREQ: usize = 10_000_000;

pub const MAX_SYSCALL_NUM: usize = 500;

// 4K
pub const PAGE_SIZE: usize = 0x1000;

pub const PAGE_SIZE_BITS: usize = 0xc;

pub const MEMORY_END: usize = 0x80800000;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub const BIG_STRIDE: u8 = u8::MAX;

pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];

use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};

const PA_WIDTH_SV39: usize = 56;
const VA_WIDTH_SV39: usize = 39;
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

///  0                   1                   2                   3           
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     Reserved      |                     PPN[2]                        |
/// |-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-|
/// |      PPN[1]     |      PPN[0]     |RSW|D|A|G|U|X|W|R|V|               
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysPageNum(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtPageNum(pub usize);

impl From<usize> for PhysAddr {
    fn from(value: usize) -> Self {
        // Use mask to reserve low 56 bits.
        Self(value & ((1 << PA_WIDTH_SV39) - 1))
    }
}

impl From<usize> for PhysPageNum {
    fn from(value: usize) -> Self {
        // Use mask to reserve low 44 bits.
        Self(value & ((1 << PPN_WIDTH_SV39) - 1))
    }
}

impl From<usize> for VirtAddr {
    fn from(value: usize) -> Self {
        // Use mask to reserve low 39 bits.
        Self(value & ((1 << VA_WIDTH_SV39) - 1))
    }
}

impl From<usize> for VirtPageNum {
    fn from(value: usize) -> Self {
        // Use mask to reserve low 27 bits.
        Self(value & ((1 << VPN_WIDTH_SV39) - 1))
    }
}

impl From<PhysAddr> for usize {
    fn from(value: PhysAddr) -> Self {
        value.0
    }
}

impl From<PhysPageNum> for usize {
    fn from(value: PhysPageNum) -> Self {
        value.0
    }
}

impl PhysAddr {
    pub fn page_offset(&self) -> usize {
        // Use mask to reserve high bits.
        self.0 & (PAGE_SIZE - 1)
    }

    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }

    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
    }
}

impl From<PhysAddr> for PhysPageNum {
    fn from(value: PhysAddr) -> Self {
        assert_eq!(value.page_offset(), 0);
        value.floor()
    }
}

impl From<PhysPageNum> for PhysAddr {
    fn from(value: PhysPageNum) -> Self {
        Self(value.0 << PAGE_SIZE_BITS)
    }
}

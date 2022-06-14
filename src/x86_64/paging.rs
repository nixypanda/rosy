use core::{fmt, ops::Index};

use bitflags::bitflags;

use super::address::PhysicalAddress;

const ENTRY_COUNT: usize = 512;

#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }
}

impl Index<PageTableIndex> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: PageTableIndex) -> &Self::Output {
        &self.entries[index.0 as usize]
    }
}

pub struct PageTableEntry {
    entry: u64,
}

bitflags! {
    pub struct PageTableEntryFlags: u64 {
        // Is the page table present in memory or not
        const PRESENT         = 1 << 0;
        // Controls whether writes to the mapped frames are allowed.
        //
        // If this bit is unset in a level 1 page table entry, the mapped frame is read-only.
        // If this bit is unset in a higher level page table entry the complete range of mapped
        // pages is read-only.
        const WRITABLE        = 1 << 1;
        // Can programs with CPL=0 execute read this value
        const USER_ACCESSIBLE = 1 << 2;
        // If set writes go directly to memory
        const WRITE_THROUGH   = 1 << 3;
        // No cache is used for this page
        const NO_CACHE        = 1 << 4;
        // CPU sets it if the mapped frame or page table is used.
        const ACCESSED        = 1 << 5;
        // CPU sets it when it performs the write to the mapped frame
        const DIRTY           = 1 << 6;
        // Specifies that the entry maps a huge frame instead of a page table. Only allowed in
        // P2 or P3 tables.
        const HUGE_PAGE       = 1 << 7;
        // Idicates this mapping is present for all address spaces. Basically a way to indicate to
        // the CPU that don't flush this page from the TLB
        const GLOBAL          = 1 << 8;
        // 9-11 and 52-62 are available for us to use as we see fit (e.g. custom flags etc)
        // Forbid code execution from this page
        const NO_EXECUTE      = 1 << 63;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameError {
    FrameNotPresent,
    HugeFrame,
}

impl PageTableEntry {
    pub fn new() -> PageTableEntry {
        PageTableEntry { entry: 0 }
    }

    pub fn flags(&self) -> PageTableEntryFlags {
        PageTableEntryFlags::from_bits_truncate(self.entry)
    }

    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.entry & 0x000f_ffff_ffff_f000)
    }

    pub fn is_unused(&self) -> bool {
        self.entry == 0
    }

    pub fn frame(&self) -> Result<PageTableFrame, FrameError> {
        if !self.flags().contains(PageTableEntryFlags::PRESENT) {
            Err(FrameError::FrameNotPresent)
        } else if self.flags().contains(PageTableEntryFlags::HUGE_PAGE) {
            Err(FrameError::HugeFrame)
        } else {
            Ok(PageTableFrame::containing_address(self.address()))
        }
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("PageTableEntry");
        f.field("addr", &self.address());
        f.field("flags", &self.flags());
        f.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct PageTableFrame {
    start_address: PhysicalAddress,
}

impl PageTableFrame {
    pub fn containing_address(address: PhysicalAddress) -> Self {
        PageTableFrame {
            start_address: address.align_down(4096),
        }
    }

    pub fn start_address(&self) -> PhysicalAddress {
        self.start_address
    }

    #[cfg(test)]
    pub fn from_raw(address: PhysicalAddress) -> Self {
        PageTableFrame {
            start_address: address,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageOffset(u16);

impl PageOffset {
    pub fn new_truncate(offset: u16) -> PageOffset {
        PageOffset(offset % (1 << 12))
    }

    #[cfg(test)]
    pub fn from_raw(index: u16) -> PageOffset {
        PageOffset(index)
    }
}

impl From<PageOffset> for u64 {
    fn from(offset: PageOffset) -> Self {
        u64::from(offset.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    pub fn new_truncate(index: u16) -> PageTableIndex {
        PageTableIndex(index % (1 << 9))
    }

    #[cfg(test)]
    pub fn from_raw(index: u16) -> PageTableIndex {
        PageTableIndex(index)
    }
}

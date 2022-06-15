use core::{fmt, marker::PhantomData, ops::Index};

use bitflags::bitflags;

use super::{
    address::{PhysicalAddress, VirtualAddress},
    instructions::read_control_register_3,
};

const ENTRY_COUNT: usize = 512;

/// Representation of a page table
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

/// 64-bit page table entry
#[repr(transparent)]
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

    pub fn has_huge_frame(&self) -> bool {
        self.flags().contains(PageTableEntryFlags::HUGE_PAGE)
    }

    /// Returns the physical frame mapped by this entry.
    ///
    /// Returns the following errors:
    ///
    /// - `FrameError::FrameNotPresent` if the entry doesn't have the `PRESENT` flag set.
    ///
    /// # Panics
    /// If the entry has the `HUGE_PAGE` flag set when the provided PageLevel is 4 or 1 it panics
    /// as this is not a valid mapping.
    pub fn frame(&self, entry_level: PageTableLevel) -> Result<MappedFrame, FrameError> {
        if !self.flags().contains(PageTableEntryFlags::PRESENT) {
            Err(FrameError::FrameNotPresent)
        } else if self.flags().contains(PageTableEntryFlags::HUGE_PAGE) {
            match entry_level {
                PageTableLevel::Level3 => Ok(MappedFrame::Huge(
                    PageTableFrame::containing_address(self.address()),
                )),
                PageTableLevel::Level2 => Ok(MappedFrame::Giant(
                    PageTableFrame::containing_address(self.address()),
                )),
                _ => panic!(
                    "Huge page is unsupported at this level {:?}. Impossible state reached",
                    entry_level,
                ),
            }
        } else {
            Ok(MappedFrame::Normal(PageTableFrame::containing_address(
                self.address(),
            )))
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

/// Represents a SIZE-bit offset into a SIZE page table.
///
/// - For `PageSize::Page4KiB` this is a 12-bit offset. Can only every contain 0-4095.
/// - For `PageSize::Page2MiB` this is a 21-bit offset. Can only every contain 0-2097151.
/// - For `PageSize::Page1GiB` this is a 30-bit offset. Can only every contain 0-1073741823.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageOffset<S>
where
    S: PageSize,
{
    offset: u32,
    _phantom: PhantomData<S>,
}

impl<S> PageOffset<S>
where
    S: PageSize,
{
    /// Create a new `PageOffset` with the given offset of `u32`. Throws away the bits if the value
    /// is
    /// >= 4096 for `PageSize::Page4KiB`
    /// >= 2097152 for `PageSize::Page2MiB`
    /// >= 1073741823 for `PageSize::Page1GiB`
    pub fn new_truncate(offset: u32) -> Self {
        PageOffset {
            offset: (offset % (1 << S::BITS)),
            _phantom: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn from_raw(index: u32) -> Self {
        PageOffset {
            offset: index,
            _phantom: PhantomData,
        }
    }
}

impl<S> From<PageOffset<S>> for u64
where
    S: PageSize,
{
    fn from(offset: PageOffset<S>) -> Self {
        u64::from(offset.offset)
    }
}

/// A 9-bit index into a page table used to access a page table entry from the 512 possible entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    pub fn new_truncate(index: u16) -> PageTableIndex {
        PageTableIndex(index % ENTRY_COUNT as u16)
    }

    #[cfg(test)]
    pub fn from_raw(index: u16) -> PageTableIndex {
        PageTableIndex(index)
    }
}

/// Trait for abstracting over the three possible page sizes on x86_64, 4KiB, 2MiB, 1GiB.
pub trait PageSize {
    const SIZE: u64;
    const BITS: usize;
}

/// Standard Page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size4KiB {}

impl PageSize for Size4KiB {
    const SIZE: u64 = 4 * 1024;
    const BITS: usize = 12;
}

/// Huge Page
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size2MiB {}

impl PageSize for Size2MiB {
    const SIZE: u64 = 2 * 1024 * 1024;
    const BITS: usize = 21;
}

/// Giant Page.
/// Only available in the newer x86_64 processors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size1GiB {}

impl PageSize for Size1GiB {
    const SIZE: u64 = 1024 * 1024 * 1024;
    const BITS: usize = 30;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameError {
    FrameNotPresent,
}

/// Physical Memory Frame
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct PageTableFrame<S>
where
    S: PageSize,
{
    start_address: PhysicalAddress,
    size: PhantomData<S>,
}

impl<S> PageTableFrame<S>
where
    S: PageSize,
{
    /// Returns the frame that contains the given address.
    pub fn containing_address(address: PhysicalAddress) -> Self {
        PageTableFrame {
            start_address: address.align_down(S::SIZE),
            size: PhantomData,
        }
    }

    pub fn start_address(&self) -> PhysicalAddress {
        self.start_address
    }

    #[cfg(test)]
    pub fn from_raw(address: PhysicalAddress) -> Self {
        PageTableFrame {
            start_address: address,
            size: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MappedFrame {
    Normal(PageTableFrame<Size4KiB>),
    Huge(PageTableFrame<Size2MiB>),
    Giant(PageTableFrame<Size1GiB>),
}

impl MappedFrame {
    pub fn start_address(&self) -> PhysicalAddress {
        match self {
            MappedFrame::Normal(frame) => frame.start_address(),
            MappedFrame::Huge(frame) => frame.start_address(),
            MappedFrame::Giant(frame) => frame.start_address(),
        }
    }

    pub fn address_at_offset(&self, offset: MappedPageOffset) -> PhysicalAddress {
        match self {
            MappedFrame::Normal(frame) => frame.start_address() + u64::from(offset),
            MappedFrame::Huge(frame) => frame.start_address() + u64::from(offset),
            MappedFrame::Giant(frame) => frame.start_address() + u64::from(offset),
        }
    }

    pub fn is_huge(&self) -> bool {
        match self {
            MappedFrame::Normal(_) => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MappedPageOffset {
    Normal(PageOffset<Size4KiB>),
    Huge(PageOffset<Size2MiB>),
    Giant(PageOffset<Size1GiB>),
}

impl From<MappedPageOffset> for u64 {
    fn from(offset: MappedPageOffset) -> Self {
        match offset {
            MappedPageOffset::Normal(offset) => u64::from(offset),
            MappedPageOffset::Huge(offset) => u64::from(offset),
            MappedPageOffset::Giant(offset) => u64::from(offset),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageTableLevel {
    Level1,
    Level2,
    Level3,
    Level4,
}

/// A Mapper implementation that requires that the complete physically memory is mapped at some
/// offset in the virtual address space.
pub struct OffsetMemoryMapper {
    physical_memory_offset: VirtualAddress,
    l4_table_address: PageTableFrame<Size4KiB>,
}

impl OffsetMemoryMapper {
    /// Creates a new `OffsetPageTable` that uses the given offset for converting virtual
    /// to physical addresses.
    ///
    /// The complete physical memory must be mapped in the virtual address space starting at
    /// address `phys_offset`. This means that for example physical address `0x5000` can be
    /// accessed through virtual address `phys_offset + 0x5000`. This mapping is required because
    /// the mapper needs to access page tables, which are not mapped into the virtual address
    /// space by default.
    ///
    /// ## Safety
    ///
    /// This function is unsafe because the caller must guarantee that the passed `phys_offset`
    /// is correct.
    pub unsafe fn new(physical_memory_offset: VirtualAddress) -> Self {
        let (level4_table_physical_address, _) = read_control_register_3();
        OffsetMemoryMapper {
            physical_memory_offset,
            l4_table_address: level4_table_physical_address,
        }
    }

    /// Return the physical address that the given virtual address is mapped to.
    ///
    /// If the given address has a valid mapping, the physical address is returned. Otherwise None
    /// is returned.
    ///
    /// This function works with huge pages of all sizes.
    ///
    /// # Panics
    /// If for some reason it detects there is a frame that is huge at level 4 or level 1 it will
    /// panic as these ase impossible states to be in
    pub fn translate_address(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        let cr3_frame = MappedFrame::Normal(self.l4_table_address);

        let l4_table = unsafe { &*(self.frame_to_pointer(cr3_frame)) };
        let entry = &l4_table[address.p4_index()];
        let l4_frame: MappedFrame = entry.frame(PageTableLevel::Level4).ok()?;

        let l3_table = unsafe { &*(self.frame_to_pointer(l4_frame)) };
        let entry = &l3_table[address.p3_index()];
        let l3_frame: MappedFrame = entry.frame(PageTableLevel::Level3).ok()?;
        if l3_frame.is_huge() {
            return Some(l3_frame.address_at_offset(address.page_offset(PageTableLevel::Level3)));
        }

        let l2_table = unsafe { &*(self.frame_to_pointer(l3_frame)) };
        let entry = &l2_table[address.p2_index()];
        let l2_frame: MappedFrame = entry.frame(PageTableLevel::Level2).ok()?;
        if l2_frame.is_huge() {
            return Some(l2_frame.address_at_offset(address.page_offset(PageTableLevel::Level2)));
        }

        let l1_table = unsafe { &*(self.frame_to_pointer(l2_frame)) };
        let entry = &l1_table[address.p1_index()];
        let l1_frame: MappedFrame = entry.frame(PageTableLevel::Level1).ok()?;

        Some(l1_frame.address_at_offset(address.page_offset(PageTableLevel::Level1)))
    }

    unsafe fn frame_to_pointer(&self, frame: MappedFrame) -> *mut PageTable {
        let virtual_address = self.physical_memory_offset + frame.start_address().as_u64();
        virtual_address.as_mut_ptr()
    }
}

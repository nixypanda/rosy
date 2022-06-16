//! Paging for x86_64.

use core::{fmt, marker::PhantomData, ops::Index};

use bitflags::bitflags;

use super::{
    address::{PhysicalAddress, VirtualAddress},
    instructions::read_control_register_3,
};

const ENTRY_COUNT: usize = 512;
const PAGE_TABLE_ENTRY_PHYSICAL_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;

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
///
/// Each page table entry has two parts to it
/// 1. The physical address of the page/frame it points to. 48 bits from 12th bit to 51st
///    (inclusive)
/// 2. The flags for the page table entry. See [`PageTableEntryFlags`]
///
/// If the page table entry has HUGE flag set and it is at Level
/// - 3, then it points to a frame that is 1GiB in size.
/// - 2, then it points to a frame that is 2MiB in size.
///
/// NOTE: HUGE flag can't be set if the entry is at Level 1 or Level 4
#[repr(transparent)]
pub struct PageTableEntry {
    entry: u64,
}

bitflags! {
    /// Flags for a page table entry
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
        PhysicalAddress::new(self.entry & PAGE_TABLE_ENTRY_PHYSICAL_ADDRESS_MASK)
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
    pub fn frame(&self, entry_level: PageTableLevel) -> Result<PageFrame, FrameError> {
        if !self.flags().contains(PageTableEntryFlags::PRESENT) {
            Err(FrameError::FrameNotPresent)
        } else if self.flags().contains(PageTableEntryFlags::HUGE_PAGE) {
            match entry_level {
                PageTableLevel::Level3 => Ok(PageFrame::Huge(PageFrameInner::containing_address(
                    self.address(),
                ))),
                PageTableLevel::Level2 => Ok(PageFrame::Giant(PageFrameInner::containing_address(
                    self.address(),
                ))),
                _ => panic!(
                    "Huge page is unsupported at this level {:?}. Impossible state reached",
                    entry_level,
                ),
            }
        } else {
            Ok(PageFrame::Normal(PageFrameInner::containing_address(
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
///
/// This is the terminal thing that a page table entry points to. It is essentially just a physical
/// address.
///
/// When using offset mapping we just add the offset obtained from the virtual address to the
/// physical start address of the frame to get the physical address that this virtual address
/// points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct PageFrameInner<S>
where
    S: PageSize,
{
    start_address: PhysicalAddress,
    size: PhantomData<S>,
}

impl<S> PageFrameInner<S>
where
    S: PageSize,
{
    /// Returns the frame that contains the given address.
    fn containing_address(address: PhysicalAddress) -> Self {
        PageFrameInner {
            start_address: address.align_down(S::SIZE),
            size: PhantomData,
        }
    }

    /// The starting address of this frame.
    fn start_address(&self) -> PhysicalAddress {
        self.start_address
    }

    #[cfg(test)]
    fn from_raw(address: PhysicalAddress) -> Self {
        PageFrameInner {
            start_address: address,
            size: PhantomData,
        }
    }
}

/// Physical Memory Frame
///
/// This is the terminal thing that a page table entry points to. It is essentially just a physical
/// address.
///
/// When using offset mapping we just add the offset obtained from the virtual address to the
/// physical start address of the frame to get the physical address that this virtual address
/// points to.
///
/// ```
/// let physical_address = PhysicalAddress::new(0x1000);
/// let virtual_address = VirtualAddress::new(0x1000);
/// let frame = PageFrame::Normal(PageFrameInner::containing_address(physical_address));
/// let mapped_physical_address = frame.convert(virtual_address);
/// ````
///
/// This can be converted to a [`PageTable`] though it is an unsafe operation (dereferencing the
/// pointer). It is the caller's job to ensure that the [`PhysicalAddress`] does indeed point to a
/// valid [`PageTable`].
///
/// ```
/// let physical_address = PhysicalAddress::new(0x1000);
/// let physical_memory_offset = VirtualAddress::new(0x1000);
/// let frame = PageFrame::Normal(PageFrameInner::containing_address(physical_address));
/// let page_table_virtual_address = physical_memory_offset + frame.start_address().as_u64();
/// let page_table = unsafe { &*(page_table_virtual_address.as_ptr() as *const PageTable) };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageFrame {
    /// PageFrame of size 4KiB with 2^9 entries
    Normal(PageFrameInner<Size4KiB>),
    /// PageFrame of size 2MiB with 2^21 entries
    Huge(PageFrameInner<Size2MiB>),
    /// PageFrame of size 1GiB with 2^30 entries
    Giant(PageFrameInner<Size1GiB>),
}

impl PageFrame {
    /// returns the starting address of this frame.
    pub fn start_address(&self) -> PhysicalAddress {
        match self {
            PageFrame::Normal(frame) => frame.start_address(),
            PageFrame::Huge(frame) => frame.start_address(),
            PageFrame::Giant(frame) => frame.start_address(),
        }
    }

    /// Converts a given [`VirtualAddress`] to a [`PhysicalAddress`].
    fn convert(&self, addr: VirtualAddress) -> PhysicalAddress {
        match self {
            PageFrame::Normal(frame) => {
                frame.start_address() + addr.page_offset(PageTableLevel::Level1)
            }
            PageFrame::Huge(frame) => {
                frame.start_address() + addr.page_offset(PageTableLevel::Level2)
            }
            PageFrame::Giant(frame) => {
                frame.start_address() + addr.page_offset(PageTableLevel::Level3)
            }
        }
    }

    /// Constucts a Level 4 PageFrame from the provided [`PhysicalAddress`].
    ///
    /// Useful to create a PageFrame when you have just read the physical address from the CR3
    /// register
    pub fn top_level_containing_address(address: PhysicalAddress) -> Self {
        PageFrame::Normal(PageFrameInner::containing_address(address))
    }

    /// Tells if this frame is huge or not
    fn is_huge(&self) -> bool {
        match self {
            PageFrame::Normal(_) => false,
            _ => true,
        }
    }

    #[cfg(test)]
    pub fn normal_from_raw(address: PhysicalAddress) -> Self {
        PageFrame::Normal(PageFrameInner::from_raw(address))
    }
}

/// Represents a SIZE-bit offset into a frame of size SIZE.
///
/// - For [`Size4KiB`] this is a 12-bit offset. Can only every contain 0-4095.
/// - For [`Size2MiB`] this is a 21-bit offset. Can only every contain 0-2097151.
/// - For [`Size1GiB`] this is a 30-bit offset. Can only every contain 0-1073741823.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageOffset {
    /// PageOffset of size 4KiB meant to index into a [`PageFrame`] with 2^9 entries
    Normal(PageOffsetInner<Size4KiB>),
    /// PageOffset of size 2MiB meant to index into a [`PageFrame`] with 2^21 entries
    Huge(PageOffsetInner<Size2MiB>),
    /// PageOffset of size 1GiB meant to index into a [`PageFrame`] with 2^30 entries
    Giant(PageOffsetInner<Size1GiB>),
}

impl From<PageOffset> for u64 {
    fn from(offset: PageOffset) -> Self {
        match offset {
            PageOffset::Normal(offset) => u64::from(offset),
            PageOffset::Huge(offset) => u64::from(offset),
            PageOffset::Giant(offset) => u64::from(offset),
        }
    }
}

/// Represents the Level at which the [`PageTable`] is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageTableLevel {
    /// Lowest level [`PageTable`]. Entries at this level exclucively map to [`PageFrame`]s.
    /// Each [`PageFrame`] has 2^9 entries.
    Level1,
    /// Second lowest level [`PageTable`]. Entries at this level either map to [`PageFrame`]s
    /// or [`PageTable`]s. Each [`PageFrame`] has 2^21 entries on the other hand each [`PageTable`]
    /// as usual has 2^9 entries.
    Level2,
    /// Third lowest level [`PageTable`]. Entries at this level either map to [`PageFrame`]s or
    /// [`PageTable`]s. Each [`PageFrame`] has 2^30 entries on the other hand each [`PageTable`] as
    /// usual has 2^9 entries.
    Level3,
    /// Highest level [`PageTable]`. Contains entries that map to [`PageTable`]s. Each
    /// [`PageTable`] as usual has 2^9 entries.
    Level4,
}

/// Represents a SIZE-bit offset into a frame of size SIZE.
///
/// - For [`Size4KiB`] this is a 12-bit offset. Can only every contain 0-4095.
/// - For [`Size2MiB`] this is a 21-bit offset. Can only every contain 0-2097151.
/// - For [`Size1GiB`] this is a 30-bit offset. Can only every contain 0-1073741823.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageOffsetInner<S>
where
    S: PageSize,
{
    offset: u32,
    _phantom: PhantomData<S>,
}

impl<S> PageOffsetInner<S>
where
    S: PageSize,
{
    /// Create a new `PageOffset` with the given offset of `u32`. Throws away the bits if the value
    /// is
    /// >= 4096 for `PageSize::Page4KiB`
    /// >= 2097152 for `PageSize::Page2MiB`
    /// >= 1073741823 for `PageSize::Page1GiB`
    pub fn new_truncate(offset: u32) -> Self {
        PageOffsetInner {
            offset: (offset % (1 << S::BITS)),
            _phantom: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn from_raw(index: u32) -> Self {
        PageOffsetInner {
            offset: index,
            _phantom: PhantomData,
        }
    }
}

impl<S> From<PageOffsetInner<S>> for u64
where
    S: PageSize,
{
    fn from(offset: PageOffsetInner<S>) -> Self {
        u64::from(offset.offset)
    }
}

/// A Mapper implementation that requires that the complete physically memory is mapped at some
/// offset in the virtual address space.
pub struct OffsetMemoryMapper {
    physical_memory_offset: VirtualAddress,
    l4_table_address: PageFrame,
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
        let cr3_frame = self.l4_table_address;

        let l4_table: &PageTable = unsafe { &*(self.frame_to_pointer(cr3_frame)) };
        let entry = &l4_table[address.p4_index()];
        let l4_frame: PageFrame = entry.frame(PageTableLevel::Level4).ok()?;

        let l3_table: &PageTable = unsafe { &*(self.frame_to_pointer(l4_frame)) };
        let entry = &l3_table[address.p3_index()];
        let l3_frame: PageFrame = entry.frame(PageTableLevel::Level3).ok()?;
        if l3_frame.is_huge() {
            return Some(l3_frame.convert(address));
        }

        let l2_table: &PageTable = unsafe { &*(self.frame_to_pointer(l3_frame)) };
        let entry = &l2_table[address.p2_index()];
        let l2_frame: PageFrame = entry.frame(PageTableLevel::Level2).ok()?;
        if l2_frame.is_huge() {
            return Some(l2_frame.convert(address));
        }

        let l1_table: &PageTable = unsafe { &*(self.frame_to_pointer(l2_frame)) };
        let entry = &l1_table[address.p1_index()];
        let l1_frame: PageFrame = entry.frame(PageTableLevel::Level1).ok()?;

        Some(l1_frame.convert(address))
    }

    /// Convert a given [`PageFrame`] to a pointer to a [`PageTable`].
    ///
    /// # Safety
    /// This function is unsafe because a [`PageFrame`] is just a [`PhysicalAddress`] in the memory
    /// and a [`PageTable`] is an array of [`PageTableEntry`]s. The caller must guarantee that the
    /// address inside the [`PageFrame`] is indeed pointing to a [`PageTable`] in the memory
    unsafe fn frame_to_pointer(&self, frame: PageFrame) -> *mut PageTable {
        let virtual_address = self.physical_memory_offset + frame.start_address().as_u64();
        virtual_address.as_mut_ptr()
    }
}

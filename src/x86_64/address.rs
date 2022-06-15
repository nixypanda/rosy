use core::{
    fmt,
    ops::{Add, Range},
};

use bit_field::BitField;

use super::paging::{MappedPageOffset, PageOffset, PageTableIndex, PageTableLevel};

/// A canonical 64-bit virtual memory address.
///
/// This is a wrapper type around an `u64`, so it is always 8 bytes, even when compiled
/// on non 64-bit systems. The
/// [`TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) trait can be used for
/// performing conversions between `u64` and `usize`.
///
/// On `x86_64`, only the 48 lower bits of a virtual address can be used. The top 16 bits need
/// to be copies of bit 47, i.e. the most significant bit. Addresses that fulfil this criterium
/// are called “canonical”. This type guarantees that it always represents a canonical address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtualAddress(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InvalidVirtualAddress(u64);

const VIRTUAL_ADDRESS_SIGN_EXTENSION_RANGE: Range<usize> = 47..64;
const OFFSET_BITS: usize = 12;
const PAGE_TABLE_INDEX_BITS: usize = 9;

const PHYSICAL_ADDRESS_NO_DATA_RANGE: Range<usize> = 52..64;

impl VirtualAddress {
    /// Creates a new canonical virtual address.
    ///
    /// This function performs sign extension of bit 47 to make the address canonical.
    ///
    /// ## Panics
    ///
    /// This function panics if the bits in the range 48 to 64 contain data (i.e. are not null and no sign extension).
    pub fn new(addr: u64) -> Self {
        Self::try_new(addr).expect(
            "address passed to VirtualAddress::new must not contain any data in bits 48 to 64",
        )
    }

    /// Tries to create a new canonical virtual address.
    ///
    /// This function tries to performs sign
    /// extension of bit 47 to make the address canonical. It succeeds if bits 48 to 64 are
    /// either a correct sign extension (i.e. copies of bit 47) or all null. Else, an error
    /// is returned.
    pub fn try_new(addr: u64) -> Result<Self, InvalidVirtualAddress> {
        match addr.get_bits(VIRTUAL_ADDRESS_SIGN_EXTENSION_RANGE) {
            0 | 0x1ffff => Ok(Self(addr)),
            1 => Ok(Self::new_truncate(addr)),
            _ => Err(InvalidVirtualAddress(addr)),
        }
    }

    /// Creates a new canonical virtual address, throwing out bits 48..64.
    ///
    /// This function performs sign extension of bit 47 to make the address canonical, so
    /// bits 48 to 64 are overwritten. If you want to check that these bits contain no data,
    /// use `new` or `try_new`.
    pub fn new_truncate(addr: u64) -> Self {
        // By doing the right shift as a signed operation (on a i64), it will
        // sign extend the value, repeating the leftmost bit.
        let len = VIRTUAL_ADDRESS_SIGN_EXTENSION_RANGE.len() - 1;
        Self(((addr << len) as i64 >> len) as u64)
    }

    #[cfg(test)]
    pub fn from_raw(address: u64) -> Self {
        Self(address)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new(ptr as u64)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    #[cfg(target_pointer_width = "64")]
    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[cfg(target_pointer_width = "64")]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }

    pub fn page_offset(self, level: PageTableLevel) -> MappedPageOffset {
        match level {
            PageTableLevel::Level1 => {
                MappedPageOffset::Normal(PageOffset::new_truncate(self.0 as u32))
            }
            PageTableLevel::Level2 => {
                MappedPageOffset::Huge(PageOffset::new_truncate(self.0 as u32))
            }
            PageTableLevel::Level3 => {
                MappedPageOffset::Huge(PageOffset::new_truncate(self.0 as u32))
            }
            PageTableLevel::Level4 => {
                panic!("VirtualAddress::page_offset: level 4 is not supported");
            }
        }
    }

    pub fn p1_index(self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.0 >> OFFSET_BITS) as u16)
    }

    pub fn p2_index(self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.0 >> OFFSET_BITS >> PAGE_TABLE_INDEX_BITS) as u16)
    }

    pub fn p3_index(self) -> PageTableIndex {
        PageTableIndex::new_truncate(
            (self.0 >> OFFSET_BITS >> PAGE_TABLE_INDEX_BITS >> PAGE_TABLE_INDEX_BITS) as u16,
        )
    }

    pub fn p4_index(self) -> PageTableIndex {
        PageTableIndex::new_truncate(
            (self.0
                >> OFFSET_BITS
                >> PAGE_TABLE_INDEX_BITS
                >> PAGE_TABLE_INDEX_BITS
                >> PAGE_TABLE_INDEX_BITS) as u16,
        )
    }
}

impl Add<u64> for VirtualAddress {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        VirtualAddress::new(self.0 + rhs as u64)
    }
}

impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("VirtualAddress")
            .field(&format_args!("{:#x?}", self.0))
            .finish()
    }
}

/// A passed `u64` was not a valid physical address.
///
/// This means that bits 52 to 64 were not all null.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysicalAddress(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InvalidPhysicalAddress(u64);

impl PhysicalAddress {
    pub fn new(addr: u64) -> Self {
        Self::try_new(addr).expect("Physical address must not contain any data in bits 52 to 64")
    }

    pub fn try_new(addr: u64) -> Result<Self, InvalidPhysicalAddress> {
        match addr.get_bits(PHYSICAL_ADDRESS_NO_DATA_RANGE) {
            0 => Ok(PhysicalAddress(addr)),
            _ => Err(InvalidPhysicalAddress(addr)),
        }
    }

    #[cfg(test)]
    pub fn from_raw(address: u64) -> Self {
        Self(address)
    }

    #[cfg(target_pointer_width = "64")]
    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[cfg(target_pointer_width = "64")]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }

    pub fn align_down(&self, alignment: u64) -> Self {
        if !alignment.is_power_of_two() {
            panic!("alignment must be a power of two");
        }
        PhysicalAddress::new(self.0 & !(alignment - 1))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Add<u64> for PhysicalAddress {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        PhysicalAddress::new(self.0 + rhs as u64)
    }
}

impl fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("PhysicalAddress")
            .field(&format_args!("{:#x?}", self.0))
            .finish()
    }
}

#[test_case]
fn test_cant_create_virtual_address_with_arbitry_bits_in_address_extension_range() {
    let invalid_virtual_address: u64 = 1 << 49;
    assert_eq!(
        VirtualAddress::try_new(invalid_virtual_address),
        Err(InvalidVirtualAddress(invalid_virtual_address))
    );
}

#[test_case]
fn test_perfoms_address_extension_when_address_is_not_canonical() {
    let address: u64 = 1 << 47;
    let valid_address: u64 = 0xff_ff_80_00_00_00_00_00;
    assert_eq!(
        VirtualAddress::try_new(address),
        Ok(VirtualAddress::from_raw(valid_address))
    );
}

#[test_case]
fn test_new_truncate_does_proper_sign_extension_zero() {
    let invalid_address: u64 = 0xff_00_00_00_00_00_00_ff;
    let valid_address: u64 = 0xff;
    assert_eq!(
        VirtualAddress::new_truncate(invalid_address),
        VirtualAddress::from_raw(valid_address)
    );
}

#[test_case]
fn test_new_truncate_does_proper_sign_extension_one() {
    let invalid_address: u64 = 0x00_f0_80_00_00_00_00_00;
    let valid_address: u64 = 0xff_ff_80_00_00_00_00_00;
    assert_eq!(
        VirtualAddress::new_truncate(invalid_address),
        VirtualAddress::from_raw(valid_address)
    );
}

#[test_case]
fn test_page_table_index_extraction_works() {
    let address: u64 = 0o001_000_777_177_2716;
    assert_eq!(
        VirtualAddress::new(address).p1_index(),
        PageTableIndex::from_raw(0o177)
    );
    assert_eq!(
        VirtualAddress::new(address).p2_index(),
        PageTableIndex::from_raw(0o777)
    );
    assert_eq!(
        VirtualAddress::new(address).p3_index(),
        PageTableIndex::from_raw(0o0)
    );
    assert_eq!(
        VirtualAddress::new(address).p4_index(),
        PageTableIndex::from_raw(0o1)
    );
}

#[test_case]
fn test_page_table_offset_exstaction_works() {
    let address: u64 = 0o001_000_777_177_2716;
    assert_eq!(
        VirtualAddress::new(address).page_offset(PageTableLevel::Level1),
        MappedPageOffset::Normal(PageOffset::from_raw(0o2716))
    );
}

#[test_case]
fn test_physical_address_creation_fails_when_there_are_bits_in_no_data_range() {
    let invalid_address: u64 = 0x00_f0_00_00_00_00_00_01;
    assert_eq!(
        PhysicalAddress::try_new(invalid_address),
        Err(InvalidPhysicalAddress(invalid_address))
    )
}

#[test_case]
fn test_physical_address_creation_for_legit_address() {
    let valid_address: u64 = 0x00_0f_00_00_00_00_00_01;
    assert_eq!(
        PhysicalAddress::try_new(valid_address),
        Ok(PhysicalAddress::from_raw(valid_address))
    )
}

#[test_case]
fn test_physical_address_align_down_gets_a_number_to_nearest_multiple_of_alignment_factor() {
    let address: u64 = 255;
    assert_eq!(
        PhysicalAddress::new(address).align_down(4),
        PhysicalAddress::from_raw(252)
    );
    assert_eq!(
        PhysicalAddress::new(address).align_down(8),
        PhysicalAddress::from_raw(248)
    );
    assert_eq!(
        PhysicalAddress::new(address).align_down(64),
        PhysicalAddress::from_raw(192)
    );
    assert_eq!(
        PhysicalAddress::new(address).align_down(256),
        PhysicalAddress::from_raw(0)
    );
}

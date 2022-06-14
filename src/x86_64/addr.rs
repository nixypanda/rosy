use core::{fmt, ops::Add};

use bit_field::BitField;

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

impl VirtualAddress {
    /// Creates a new canonical virtual address.
    ///
    /// This function performs sign extension of bit 47 to make the address canonical.
    ///
    /// ## Panics
    ///
    /// This function panics if the bits in the range 48 to 64 contain data (i.e. are not null and no sign extension).
    pub fn new(addr: u64) -> VirtualAddress {
        Self::try_new(addr)
            .expect("address passed to VirtAddr::new must not contain any data in bits 48 to 64")
    }

    /// Tries to create a new canonical virtual address.
    ///
    /// This function tries to performs sign
    /// extension of bit 47 to make the address canonical. It succeeds if bits 48 to 64 are
    /// either a correct sign extension (i.e. copies of bit 47) or all null. Else, an error
    /// is returned.
    pub fn try_new(addr: u64) -> Result<VirtualAddress, InvalidVirtualAddress> {
        match addr.get_bits(47..64) {
            0 | 0x1ffff => Ok(VirtualAddress(addr)), // address is canonical
            1 => Ok(VirtualAddress::new_truncate(addr)), // address needs sign extension
            other => Err(InvalidVirtualAddress(other)),
        }
    }

    /// Creates a new canonical virtual address, throwing out bits 48..64.
    ///
    /// This function performs sign extension of bit 47 to make the address canonical, so
    /// bits 48 to 64 are overwritten. If you want to check that these bits contain no data,
    /// use `new` or `try_new`.
    pub const fn new_truncate(addr: u64) -> VirtualAddress {
        // By doing the right shift as a signed operation (on a i64), it will
        // sign extend the value, repeating the leftmost bit.
        VirtualAddress(((addr << 16) as i64 >> 16) as u64)
    }

    pub fn zero() -> VirtualAddress {
        VirtualAddress(0)
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
}

impl Add<usize> for VirtualAddress {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
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
    pub fn new(addr: u64) -> PhysicalAddress {
        Self::try_new(addr).expect("Physical address must not contain any data in bits 52 to 64")
    }

    pub fn try_new(addr: u64) -> Result<PhysicalAddress, InvalidPhysicalAddress> {
        match addr.get_bits(52..64) {
            0 => Ok(PhysicalAddress(addr)),
            other => Err(InvalidPhysicalAddress(other)),
        }
    }

    #[cfg(target_pointer_width = "64")]
    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[cfg(target_pointer_width = "64")]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }

    pub fn align_down(&self, alignment: u64) -> PhysicalAddress {
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

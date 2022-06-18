//! Various heap allocation strategies

pub mod bump;
pub mod linked_list;

/// Align the given address `address` to the next multiple of `align`.
fn align_up(address: usize, align: usize) -> usize {
    let remainder = address % align;
    if remainder == 0 {
        address // addr already aligned
    } else {
        address - remainder + align
    }
}

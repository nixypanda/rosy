//! Linked List Allocator design

use core::{
    alloc::{GlobalAlloc, Layout},
    mem, ptr,
};

use crate::{allocator::align_up, utils::Locked};

struct ListNode {
    next: Option<&'static mut ListNode>,
    size: usize,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// Keeps track of free space in a single Linked List.
///
/// Each node in this linked list represents free space in the memory. Each node contains size of
/// the memory and the pointer to the next available free space. We only need the pointer to the
/// first unused region to keep track of all the unused regions. This is often referred to as free
/// list.
///
/// ```text
/// Legend:
/// AR -> Allocated Region
/// FR -> Free Region
/// sn -> List Node with s being size and n being pointer to the next node
///     ┌────────────────────────────┐
///     │                            ▼
///  ┌──┴────────────────┬───────────┬────────────┬────────────┬────────────────────────────┐
///  │ sn    FR          │    AR     │ sn  FR     │   AR       │ sn  FR                     │
///  └───────────────────┴───────────┴──┬─────────┴────────────┴────────────────────────────┘
///  ▲                                  │                       ▲
///  └───head                           └───────────────────────┘
///
/// ```
///
/// # Operations
///
/// * *Adding a free region*: When an allocated region is freed we add it to the top of the head.
/// (This operation is essentially like adding a head to a linked list).
/// * *Allocationg a region*: We traverse through the list to find an appropriate region and then
/// assign a portion of that region. We update the pointers accordingly. (This operation is like
/// deleting a node and then optionally inserting a new one in it's place.)
///
/// # Limitation(s)
/// * Currently we don't merge free blocks together this leads to fragmentation of memory.
/// Eventually even if we free up everything we can't allocate space even if we have that much free
/// space available with us. A more robust implementation will need to merge these adjacent free
/// blocks together.
/// * We need to traverse the whole list in order to find a suitable region to allocate. This can
/// be a performance nightmare.
pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        LinkedListAllocator {
            head: ListNode::new(0),
        }
    }

    /// Initialize the allocator with the heap information
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the given heap bounds are
    /// valid and that the heap is unused. This method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_end: usize) {
        self.add_free_region(heap_start, heap_end);
    }

    unsafe fn add_free_region(&mut self, address: usize, size: usize) {
        // ensure that the freed region is capable of holding ListNode
        assert_eq!(align_up(address, mem::align_of::<ListNode>()), address);
        assert!(size >= mem::size_of::<ListNode>());

        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = address as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr);
    }

    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(allocation_start) = Self::allocate_from_region(&region, size, align) {
                // remove node from the list
                let next = region.next.take();
                let found_region = Some((current.next.take().unwrap(), allocation_start));
                current.next = next;
                return found_region;
            } else {
                current = current.next.as_mut().unwrap();
            }
        }

        None
    }

    /// Try to use the given region for an allocation with given size and
    /// alignment.
    ///
    /// Returns the allocation start address on success.
    fn allocate_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // region too small
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // rest of region too small to hold a ListNode (required because the
            // allocation splits the region in a used and a free part)
            return Err(());
        }

        // region suitable for allocation
        Ok(alloc_start)
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a `ListNode`.
    ///
    /// Returns the adjusted size and alignment as a (size, align) tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // perform layout adjustments
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // perform layout adjustments
        let (size, _) = LinkedListAllocator::size_align(layout);

        self.lock().add_free_region(ptr as usize, size)
    }
}

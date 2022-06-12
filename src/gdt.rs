use lazy_static::lazy_static;

use crate::x86_64::{
    addr::VirtualAddress,
    descriptor::Descriptor,
    gdt::GlobalDescriptorTable,
    segmentation::{set_code_segment_selector, SegmentSelector},
    tss::{load_task_state_segment, TaskStateSegment},
};

pub const INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT: u16 = 0;

lazy_static! {
    static ref TASK_STATE_SEGMENT: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT as usize] = {
            // We haven’t implemented memory management yet, so we don’t have a proper way to
            // allocate a new stack. Instead, we use a static mut array as stack storage for now.
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtualAddress::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}

lazy_static! {
    static ref GLOBAL_DESCRIPTOR_TABLE: (GlobalDescriptorTable, SegmentSelector, SegmentSelector) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TASK_STATE_SEGMENT));

        (gdt, code_selector, tss_selector)
    };
}

pub fn init() {
    GLOBAL_DESCRIPTOR_TABLE.0.load();
    unsafe {
        set_code_segment_selector(GLOBAL_DESCRIPTOR_TABLE.1);
        load_task_state_segment(GLOBAL_DESCRIPTOR_TABLE.2);
    }
}

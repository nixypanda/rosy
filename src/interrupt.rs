use lazy_static::lazy_static;

use crate::{
    gdt::INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT,
    pic8258::ChainedPics,
    print, println,
    x86_64::{
        idt::{ExceptionStackFrame, InterruptDescriptorTable, PageFaultErrorCode},
        instructions::read_control_register_2,
        port::Port,
    },
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

const PS_2_CONTROLLER_PORT: u16 = 0x60;

lazy_static! {
    pub static ref INTERRUPT_DESCRIPTOR_TABLE: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.set_breakpoint_handler(breakpoint_handler);
        unsafe {
            idt.set_double_fault_handler(double_fault_handler)
                .set_stack_index(INTERRUPT_STACK_TABLE_INDEX_DOUBLE_FAULT);
        }
        idt.set_page_fault_handler(page_fault_handler);
        idt.set_hardware_interrupt(InterruptIndex::Timer.as_u8(), timer_interrupt_handler);
        idt.set_hardware_interrupt(InterruptIndex::Keyboard.as_u8(), keyboard_interrupt_handler);
        idt
    };
}

lazy_static! {
    pub static ref PROGRAMABLE_INTERRUPT_CONTROLERS: spin::Mutex<ChainedPics> =
        spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: ExceptionStackFrame) {
    println!("EXCEPTION: BREAKPOINT ERROR\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: ExceptionStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: ExceptionStackFrame,
    error_code: PageFaultErrorCode,
) {
    let responsible_virtual_address = read_control_register_2();

    println!("EXCEPTION: PAGE FAULT");
    println!("EXCEPTION: PAGE FAULT: Error Code: {:?}", error_code);
    println!(
        "EXCEPTION: PAGE FAULT: Virtual address responsible {:?}",
        responsible_virtual_address
    );
    println!("EXCEPTION: PAGE FAULT: Stack Frame\n{:#?}", stack_frame);
    loop {}
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: ExceptionStackFrame) {
    unsafe {
        PROGRAMABLE_INTERRUPT_CONTROLERS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: ExceptionStackFrame) {
    use crate::keyboard::ColemakDHm;
    use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<ColemakDHm, ScancodeSet1>> = Mutex::new(Keyboard::new(
            ColemakDHm,
            ScancodeSet1,
            HandleControl::Ignore
        ));
    }

    let mut keyboard = KEYBOARD.lock();
    let port = Port::new(PS_2_CONTROLLER_PORT);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PROGRAMABLE_INTERRUPT_CONTROLERS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

pub fn init() {
    INTERRUPT_DESCRIPTOR_TABLE.load();
}

pub fn invoke_page_fault_exception() {
    unsafe {
        *(0xdeadbeef as *mut u64) = 42;
    };
}

#[allow(unconditional_recursion)]
pub fn stack_overflow() {
    stack_overflow();
}

#[test_case]
fn test_breakpoint_exception() {
    use crate::x86_64::interrupts::invoke_breakpoint_exception;
    // Execution continues => Breakpoint handler is working
    invoke_breakpoint_exception();
}

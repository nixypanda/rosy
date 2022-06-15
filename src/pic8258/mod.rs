//! Implementation of Intel 8258 PIC.
//!
//! The Intel 8259 is a programmable interrupt controller (PIC) introduced in 1976. It has long
//! been replaced by the newer APIC, but its interface is still supported on current systems for
//! backwards compatibility reasons. The 8259 PIC is significantly easier to set up than the APIC.
//! So we have it here

use crate::x86_64::port::Port;

const NUMBER_OF_PINS: u8 = 8;
const CMD_END_OF_INTERRUPT: u8 = 0x20;

const IO_BASE_ADDRESS_PRIMARY: u16 = 0x20;
const IO_BASE_ADDRESS_SECONDARY: u16 = 0xA0;

const COMMAND_PORT_PRIMARY: u16 = IO_BASE_ADDRESS_PRIMARY;
const DATA_PORT_PRIMARY: u16 = IO_BASE_ADDRESS_PRIMARY + 1;
const COMMAND_PORT_SECONDARY: u16 = IO_BASE_ADDRESS_SECONDARY;
const DATA_PORT_SECONDARY: u16 = IO_BASE_ADDRESS_SECONDARY + 1;

const ANY_UNUSED_PORT: u16 = 0x80;

const PIC_INITIALIZATION_COMMAND: u8 = 0x11;

const SECONDARY_PIC_ADDRESS: u8 = 0x04;
const SECONDARY_PIC_CASCADE_IDENTITY: u8 = 0x02;

const MODE_8086: u8 = 0x01;

const GARBAGE_VALUE: u8 = 0x00;

struct ProgrammableInterrupController {
    // The default offset used by PIC is 0 but this spot is reserved for the exceptions handlers.
    // So we need to use a different offset. Usually this number is 0x20 (or 32) but we are keeping
    // this configurable.
    offset: u8,
    data: Port<u8>,
    command: Port<u8>,
}

// Idividual 8259 programmable interrupt controller
impl ProgrammableInterrupController {
    fn handles_this_interrupt(&self, interrupt_id: u8) -> bool {
        interrupt_id >= self.offset && interrupt_id < self.offset + NUMBER_OF_PINS
    }

    unsafe fn write_end_of_interrupt(&self) {
        self.command.write(CMD_END_OF_INTERRUPT);
    }

    unsafe fn read_mask(&mut self) -> u8 {
        self.data.read()
    }

    unsafe fn write_mask(&mut self, mask: u8) {
        self.data.write(mask)
    }
}

/// Represents chained 8259 PICs.
///
/// The 8259 has 8 interrupt lines and several lines for communicating with the CPU. The typical
/// systems back then were equipped with two instances of the 8259 PIC, one primary and one
/// secondary PIC connected to one of the interrupt lines of the primary:
///
/// ```text
///                      ____________                          ____________
/// Real Time Clock --> |            |   Timer -------------> |            |
/// ACPI -------------> |            |   Keyboard-----------> |            |      _____
/// Available --------> | Secondary  |----------------------> | Primary    |     |     |
/// Available --------> | Interrupt  |   Serial Port 2 -----> | Interrupt  |---> | CPU |
/// Mouse ------------> | Controller |   Serial Port 1 -----> | Controller |     |_____|
/// Co-Processor -----> |            |   Parallel Port 2/3 -> |            |
/// Primary ATA ------> |            |   Floppy disk -------> |            |
/// Secondary ATA ----> |____________|   Parallel Port 1----> |____________|
/// ```
///
/// This graphic shows the typical assignment of interrupt lines. We see that most of the 15 lines
/// have a fixed mapping, e.g. line 4 of the secondary PIC is assigned to the mouse.
pub struct ChainedPics {
    primary: ProgrammableInterrupController,
    secondary: ProgrammableInterrupController,
}

impl ChainedPics {
    pub unsafe fn new(offset_primary: u8, offset_secondary: u8) -> ChainedPics {
        ChainedPics {
            primary: ProgrammableInterrupController {
                offset: offset_primary,
                command: Port::new(COMMAND_PORT_PRIMARY),
                data: Port::new(DATA_PORT_PRIMARY),
            },
            secondary: ProgrammableInterrupController {
                offset: offset_secondary,
                command: Port::new(COMMAND_PORT_SECONDARY),
                data: Port::new(DATA_PORT_SECONDARY),
            },
        }
    }

    /// Initialize both our PICs.  We initialize them together, at the same time, because it's
    /// traditional to do so, and because I/O operations might not be instantaneous on older
    /// processors.
    pub unsafe fn initialize(&mut self) {
        let (primary_mask, secondary_mask) = self.read_masks();
        self.start_initialize_sequence();
        self.setup_base_offset();
        self.chain_primary_and_secondary();
        self.setup_mode();
        self.write_masks(primary_mask, secondary_mask);
    }

    // Prepares the PICs to receive 3 bytes of initialization sequence on their data ports. The
    // functions `setup_base_offset`, `chain_primary_and_secondary` and `setup_mode` handle the
    // sequence that we will send to the PICs.
    unsafe fn start_initialize_sequence(&self) {
        self.primary.command.write(PIC_INITIALIZATION_COMMAND);
        wait_a_few_microseconds();
        self.secondary.command.write(PIC_INITIALIZATION_COMMAND);
        wait_a_few_microseconds();
    }

    unsafe fn setup_base_offset(&self) {
        self.primary.data.write(self.primary.offset);
        wait_a_few_microseconds();
        self.secondary.data.write(self.secondary.offset);
        wait_a_few_microseconds();
    }

    unsafe fn chain_primary_and_secondary(&self) {
        self.primary.data.write(SECONDARY_PIC_ADDRESS);
        wait_a_few_microseconds();
        self.secondary.data.write(SECONDARY_PIC_CASCADE_IDENTITY);
        wait_a_few_microseconds();
    }

    unsafe fn setup_mode(&self) {
        self.primary.data.write(MODE_8086);
        wait_a_few_microseconds();
        self.secondary.data.write(MODE_8086);
        wait_a_few_microseconds();
    }

    /// Reads the interrupt masks of both PICs.
    pub unsafe fn read_masks(&mut self) -> (u8, u8) {
        (self.primary.read_mask(), self.secondary.read_mask())
    }

    /// Writes the interrupt masks of both PICs.
    pub unsafe fn write_masks(&mut self, primary_mask: u8, secondary_mask: u8) {
        self.primary.write_mask(primary_mask);
        self.secondary.write_mask(secondary_mask);
    }

    /// Figure out which (if any) PICs in our chain need to know about this
    /// interrupt.
    pub unsafe fn notify_end_of_interrupt(&self, interrupt_id: u8) {
        if self.primary.handles_this_interrupt(interrupt_id) {
            self.primary.write_end_of_interrupt();
        } else if self.secondary.handles_this_interrupt(interrupt_id) {
            self.secondary.write_end_of_interrupt();
            // NOTE: Informing the prigary PIC is intentional
            self.primary.write_end_of_interrupt();
        }
    }
}

// Wait a very small amount of time (1 to 4 microseconds, generally). Useful for implementing a
// small delay for PIC remapping on old hardware or generally as a simple but imprecise wait.
unsafe fn wait_a_few_microseconds() {
    let port = Port::new(ANY_UNUSED_PORT);
    port.write(GARBAGE_VALUE);
}

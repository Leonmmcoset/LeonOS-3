use alloc::format;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::logger::serial_write;

static mut IDT: Option<InterruptDescriptorTable> = None;
static mut IDT_READY: bool = false;

pub fn init() {
    unsafe {
        if IDT_READY {
            return;
        }

        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);

        IDT = Some(idt);
        if let Some(idt_ref) = IDT.as_ref() {
            idt_ref.load();
        }

        IDT_READY = true;
    }

    serial_write("[LeonOS3] traps: idt loaded\n");
}

extern "x86-interrupt" fn breakpoint_handler(stack: InterruptStackFrame) {
    serial_write(&format!(
        "[LeonOS3] trap: breakpoint rip=0x{:x}\n",
        stack.instruction_pointer.as_u64()
    ));
}

extern "x86-interrupt" fn invalid_opcode_handler(stack: InterruptStackFrame) {
    serial_write(&format!(
        "[LeonOS3] trap: invalid opcode rip=0x{:x}\n",
        stack.instruction_pointer.as_u64()
    ));
    halt_forever();
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack: InterruptStackFrame,
    code: u64,
) {
    serial_write(&format!(
        "[LeonOS3] trap: gpf rip=0x{:x} code=0x{:x}\n",
        stack.instruction_pointer.as_u64(),
        code
    ));
    halt_forever();
}

extern "x86-interrupt" fn page_fault_handler(
    stack: InterruptStackFrame,
    code: PageFaultErrorCode,
) {
    let addr = match Cr2::read() {
        Ok(v) => v.as_u64(),
        Err(v) => v.0,
    };
    serial_write(&format!(
        "[LeonOS3] trap: page fault rip=0x{:x} cr2=0x{:x} err={:?}\n",
        stack.instruction_pointer.as_u64(),
        addr,
        code
    ));
    halt_forever();
}

extern "x86-interrupt" fn double_fault_handler(
    stack: InterruptStackFrame,
    code: u64,
) -> ! {
    serial_write(&format!(
        "[LeonOS3] trap: double fault rip=0x{:x} code=0x{:x}\n",
        stack.instruction_pointer.as_u64(),
        code
    ));
    halt_forever()
}

fn halt_forever() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}



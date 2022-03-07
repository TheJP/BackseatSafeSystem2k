use crate::{memory::Memory, processor::Processor, terminal, Instruction, Size, OPCODE_LENGTH};
use raylib::prelude::*;

pub struct Machine {
    pub memory: Memory,
    pub processor: Processor,
}

impl Machine {
    pub fn new() -> Self {
        Self {
            memory: Memory::new(),
            processor: Processor::new(),
        }
    }

    pub fn render(&mut self, draw_handle: &mut RaylibDrawHandle, font: &Font) {
        terminal::render(&mut self.memory, draw_handle, Vector2::zero(), font, 20.0);
    }

    pub fn make_tick(&mut self) {
        self.processor.make_tick(&mut self.memory);
    }

    #[must_use = "Am I a joke to you?"]
    pub fn is_halted(&self) -> bool {
        let instruction = self.read_instruction_at_instruction_pointer();
        let bitmask = !(Instruction::MAX >> OPCODE_LENGTH);
        (instruction & bitmask) >> (Instruction::SIZE * 8 - OPCODE_LENGTH) == 0x0006
    }

    fn read_instruction_at_instruction_pointer(&self) -> Instruction {
        self.memory
            .read_instruction(self.processor.registers[Processor::INSTRUCTION_POINTER])
    }
}

#[cfg(test)]
mod tests {
    use crate::processor::Flag;
    use crate::{
        opcodes::Opcode::{self, *},
        Register,
    };
    use crate::{Address, Instruction, Size, Word};

    use super::*;

    #[test]
    fn make_tick_increases_instruction_pointer() {
        use crate::Size;
        let mut machine = Machine::new();
        assert_eq!(
            machine.processor.registers[Processor::INSTRUCTION_POINTER],
            Processor::ENTRY_POINT
        );
        machine.processor.make_tick(&mut machine.memory);
        assert_eq!(
            machine.processor.registers[Processor::INSTRUCTION_POINTER],
            Processor::ENTRY_POINT + Instruction::SIZE as u32
        );
    }

    fn create_machine_with_data_at(address: Address, data: Word) -> Machine {
        let mut machine = Machine::new();
        machine.memory.write_data(address, data);
        machine
    }

    fn create_machine_with_instructions(opcodes: &[Opcode]) -> Machine {
        let mut machine = Machine::new();
        for (&opcode, address) in opcodes
            .iter()
            .zip((Processor::ENTRY_POINT..).step_by(Instruction::SIZE))
        {
            machine
                .memory
                .write_instruction(address, opcode.as_instruction());
        }
        machine
    }

    fn execute_instruction_with_machine(mut machine: Machine, opcode: Opcode) -> Machine {
        let instruction_pointer = machine.processor.registers[Processor::INSTRUCTION_POINTER];
        machine
            .memory
            .write_instruction(instruction_pointer, opcode.as_instruction());
        machine.processor.make_tick(&mut machine.memory);
        assert_eq!(
            machine.processor.registers[Processor::INSTRUCTION_POINTER],
            instruction_pointer + Instruction::SIZE as u32
        );
        machine
    }

    fn execute_instruction(opcode: Opcode) -> Machine {
        execute_instruction_with_machine(Machine::new(), opcode)
    }

    #[test]
    fn move_constant_into_register() {
        let register = 0x0A.into();
        let value = 0xABCD_1234;
        let machine = execute_instruction(MoveRegisterImmediate {
            r: register,
            immediate: value,
        });
        assert_eq!(machine.processor.registers[register], value);
    }

    #[test]
    fn move_from_address_into_register() {
        let address = 0xF0;
        let data = 0xABCD_1234;
        let register = 0x0A.into();
        let machine = create_machine_with_data_at(address, data);
        let machine = execute_instruction_with_machine(
            machine,
            MoveRegisterAddress {
                r: register,
                address,
            },
        );
        assert_eq!(machine.processor.registers[register], data);
    }

    #[test]
    fn move_from_one_register_to_another() {
        let mut machine = Machine::new();
        let source = 0x5.into();
        let target = 0x0A.into();
        let data = 0xCAFE;
        machine.processor.registers[source] = data;
        let machine = execute_instruction_with_machine(
            machine,
            MoveTargetSource {
                t: target,
                s: source,
            },
        );
        assert_eq!(machine.processor.registers[target], data);
    }

    #[test]
    fn move_from_register_into_memory() {
        let mut machine = Machine::new();
        let register = 0x5.into();
        let data = 0xC0FFEE;
        let address = 0xF0;
        machine.processor.registers[register] = data;
        let machine = execute_instruction_with_machine(
            machine,
            MoveAddressRegister {
                address,
                r: register,
            },
        );
        assert_eq!(machine.memory.read_data(address), data);
    }

    #[test]
    fn move_from_memory_addressed_by_register_into_another_register() {
        let address = 0xF0;
        let data = 0xC0FFEE;
        let target = 0x0A.into();
        let pointer = 0x05.into();
        let mut machine = create_machine_with_data_at(address, data);
        machine.processor.registers[pointer] = address;
        let machine = execute_instruction_with_machine(
            machine,
            MoveTargetPointer {
                t: target,
                p: pointer,
            },
        );
        assert_eq!(machine.processor.registers[target], data);
    }

    #[test]
    fn move_from_memory_addressed_by_register_into_same_register() {
        let address = 0xF0;
        let data = 0xC0FFEE;
        let register = 0x05.into();
        let mut machine = create_machine_with_data_at(address, data);
        machine.processor.registers[register] = address;
        let machine = execute_instruction_with_machine(
            machine,
            MoveTargetPointer {
                t: register,
                p: register,
            },
        );
        assert_eq!(machine.processor.registers[register], data);
    }

    #[test]
    fn move_from_register_into_memory_addressed_by_another_register() {
        let data = 0xC0FFEE;
        let address = 0xF0;
        let pointer = 0x0A.into();
        let source = 0x05.into();
        let mut machine = Machine::new();
        machine.processor.registers[source] = data;
        machine.processor.registers[pointer] = address;
        let machine = execute_instruction_with_machine(
            machine,
            MovePointerSource {
                p: pointer,
                s: source,
            },
        );
        assert_eq!(machine.memory.read_data(address), data);
    }

    #[test]
    fn move_from_register_into_memory_addressed_by_same_register() {
        let address = 0xF0;
        let register = 0x05.into();
        let mut machine = Machine::new();
        machine.processor.registers[register] = address;
        let machine = execute_instruction_with_machine(
            machine,
            MovePointerSource {
                p: register,
                s: register,
            },
        );
        assert_eq!(machine.memory.read_data(address), address);
    }

    #[test]
    fn halt_and_catch_fire_prevents_further_instructions() {
        let register = 0x05.into();
        let value = 0x0000_0042;
        let mut machine = create_machine_with_instructions(&[
            HaltAndCatchFire {},
            MoveRegisterImmediate {
                r: register,
                immediate: value,
            },
        ]);
        for _ in 0..2 {
            machine.make_tick();
        }
        assert_eq!(
            machine.processor.registers[Processor::INSTRUCTION_POINTER],
            Processor::ENTRY_POINT
        );
        assert_eq!(machine.processor.registers[register], 0x0);
    }

    #[test]
    fn add_two_values_with_no_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = 10;
        let rhs = 12;
        let expected = lhs + rhs;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            AddTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn add_two_values_with_only_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = 0;
        let rhs = 0;
        let expected = lhs + rhs;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            AddTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn add_two_values_with_only_carry_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = Word::MAX;
        let rhs = 5;
        let expected = lhs.wrapping_add(rhs);
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            AddTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn add_two_values_with_both_zero_and_carry_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = Word::MAX;
        let rhs = 1;
        let expected = lhs.wrapping_add(rhs);
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            AddTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn subtract_two_values_with_no_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = 10;
        let rhs = 8;
        let expected = lhs - rhs;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            SubtractTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn subtract_two_values_with_only_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = 10;
        let rhs = 10;
        let expected = lhs - rhs;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            SubtractTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn subtract_two_values_with_only_carry_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 10;
        let rhs = 12;
        let expected = lhs.wrapping_sub(rhs);
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            SubtractTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn subtract_two_values_with_carry_with_no_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 14;
        let rhs = 12;
        let expected = lhs.wrapping_sub(rhs + 1 /* carry */);
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            SubtractWithCarryTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn subtract_two_values_with_carry_with_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 14;
        let rhs = 13;
        let expected = lhs.wrapping_sub(rhs + 1 /* carry */);
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            SubtractWithCarryTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn subtract_two_values_with_carry_with_both_carry_and_zero_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 0;
        let rhs = Word::MAX;
        let expected = lhs.wrapping_sub(rhs).wrapping_sub(1);
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            SubtractWithCarryTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn multiply_two_values_without_any_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_high = 0x09.into();
        let target_low = 0x0A.into();
        let lhs: Word = 3;
        let rhs = 4;
        let expected = lhs * rhs;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            MultiplyHighLowLhsRhs {
                h: target_high,
                t: target_low,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_high], 0);
        assert_eq!(machine.processor.registers[target_low], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn multiply_two_values_with_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_high = 0x09.into();
        let target_low = 0x0A.into();
        let lhs: Word = 3;
        let rhs = 0;
        let expected = lhs * rhs;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            MultiplyHighLowLhsRhs {
                h: target_high,
                t: target_low,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_high], 0);
        assert_eq!(machine.processor.registers[target_low], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn multiply_two_values_with_overflow() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_high = 0x09.into();
        let target_low = 0x0A.into();
        let lhs: Word = Word::MAX;
        let rhs = 5;
        let result = lhs as u64 * rhs as u64;
        let high_expected = (result >> 32) as u32;
        let low_expected = result as u32;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            MultiplyHighLowLhsRhs {
                h: target_high,
                t: target_low,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_high], high_expected);
        assert_eq!(machine.processor.registers[target_low], low_expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn multiply_two_values_with_overflow_and_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_high = 0x09.into();
        let target_low = 0x0A.into();
        let lhs: Word = 1 << (Word::BITS - 1);
        let rhs = 2;
        let result = lhs as u64 * rhs as u64;
        let high_expected = (result >> 32) as u32;
        let low_expected = result as u32;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            MultiplyHighLowLhsRhs {
                h: target_high,
                t: target_low,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_high], high_expected);
        assert_eq!(machine.processor.registers[target_low], low_expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn divmod_two_values_with_no_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_quotient = 0x09.into();
        let target_remainder = 0x0A.into();
        let lhs: Word = 15;
        let rhs = 6;
        let expected_quotient = 2;
        let expected_remainder = 3;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            DivmodTargetModLhsRhs {
                d: target_quotient,
                m: target_remainder,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(
            machine.processor.registers[target_quotient],
            expected_quotient
        );
        assert_eq!(
            machine.processor.registers[target_remainder],
            expected_remainder
        );
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::DivideByZero), false);
    }

    #[test]
    fn divmod_two_values_with_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_quotient = 0x09.into();
        let target_remainder = 0x0A.into();
        let lhs: Word = 0;
        let rhs = 6;
        let expected_quotient = 0;
        let expected_remainder = 0;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            DivmodTargetModLhsRhs {
                d: target_quotient,
                m: target_remainder,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(
            machine.processor.registers[target_quotient],
            expected_quotient
        );
        assert_eq!(
            machine.processor.registers[target_remainder],
            expected_remainder
        );
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::DivideByZero), false);
    }

    #[test]
    fn divmod_two_values_divide_by_zero() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_quotient = 0x09.into();
        let target_remainder = 0x0A.into();
        let lhs: Word = 15;
        let rhs = 0;
        let expected_quotient = 0;
        let expected_remainder = 15;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            DivmodTargetModLhsRhs {
                d: target_quotient,
                m: target_remainder,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(
            machine.processor.registers[target_quotient],
            expected_quotient
        );
        assert_eq!(
            machine.processor.registers[target_remainder],
            expected_remainder
        );
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::DivideByZero), true);
    }

    #[test]
    fn bitwise_and_two_values_with_no_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 0b01101110_10011010_01101110_10011010;
        let rhs = 0b10111010_01011001_10111010_01011001;
        let expected = 0b00101010_00011000_00101010_00011000;
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            AndTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
    }

    #[test]
    fn bitwise_and_two_values_with_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 0b01000100_10000110_01000100_10000010;
        let rhs = 0b10111010_01011001_10111010_01011001;
        let expected = 0;
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            AndTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
    }

    #[test]
    fn bitwise_or_two_values_with_no_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 0b01101110_10011010_01101110_10011010;
        let rhs = 0b10111010_01011001_10111010_01011001;
        let expected = 0b11111110_11011011_11111110_11011011;
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            OrTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
    }

    #[test]
    fn bitwise_or_two_values_with_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 0;
        let rhs = 0;
        let expected = 0;
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            OrTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
    }

    #[test]
    fn bitwise_xor_two_values_with_no_flags_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 0b01101110_10011010_01101110_10011010;
        let rhs = 0b10111010_01011001_10111010_01011001;
        let expected = 0b11010100_11000011_11010100_11000011;
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            XorTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
    }

    #[test]
    fn bitwise_xor_two_values_with_zero_flag_set() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs: Word = 0b10111010_10010010_01000100_10010010;
        let rhs = 0b10111010_10010010_01000100_10010010;
        let expected = 0;
        let mut machine = Machine::new();
        machine.processor.set_flag(Flag::Carry, true);
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            XorTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
    }

    #[test]
    fn bitwise_not_value_with_no_flags_set() {
        let mut machine = Machine::new();
        let source = 0x5.into();
        let target = 0x0A.into();
        let data = 0b00101010_00011000_00101010_00011000;
        let expected = 0b11010101_11100111_11010101_11100111;
        machine.processor.registers[source] = data;
        let machine = execute_instruction_with_machine(
            machine,
            NotTargetSource {
                t: target,
                s: source,
            },
        );
        assert_eq!(machine.processor.registers[target], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
    }

    #[test]
    fn bitwise_not_value_with_zero_flag_set() {
        let mut machine = Machine::new();
        let source = 0x5.into();
        let target = 0x0A.into();
        let data = 0xFFFFFFFF;
        let expected = 0;
        machine.processor.registers[source] = data;
        let machine = execute_instruction_with_machine(
            machine,
            NotTargetSource {
                t: target,
                s: source,
            },
        );
        assert_eq!(machine.processor.registers[target], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
    }

    #[test]
    fn left_shift_without_any_flags_set() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0b1;
        let rhs = 2;
        let expected = 0b100;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            LeftShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn left_shift_with_carry_flag_set() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0b11 << 30;
        let rhs = 1;
        let expected = 0b1 << 31;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            LeftShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn left_shift_with_carry_and_zero_flags_set() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0b1 << 31;
        let rhs = 1;
        let expected = 0;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            LeftShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn left_shift_way_too_far() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0xFFFF_FFFF;
        let rhs = 123;
        let expected = 0;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            LeftShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn left_shift_zero_way_too_far() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0;
        let rhs = 123;
        let expected = 0;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            LeftShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn right_shift_without_any_flags_set() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0b10;
        let rhs = 1;
        let expected = 0b1;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            RightShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn right_shift_with_carry_flag_set() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0b11;
        let rhs = 1;
        let expected = 0b1;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            RightShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn right_shift_with_zero_flag_set() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0b0;
        let rhs = 1;
        let expected = 0b0;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            RightShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn right_shift_with_carry_and_zero_flags_set() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0b1;
        let rhs = 1;
        let expected = 0b0;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            RightShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn right_shift_way_too_far() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0xFFFF_FFFF;
        let rhs = 123;
        let expected = 0b0;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            RightShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn right_shift_zero_way_too_far() {
        let mut machine = Machine::new();
        let lhs_register = 0x5.into();
        let rhs_register = 0x6.into();
        let target_register = 0x0A.into();
        let lhs = 0;
        let rhs = 123;
        let expected = 0b0;
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let machine = execute_instruction_with_machine(
            machine,
            RightShiftTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn add_immediate_with_no_flags_set() {
        let mut machine = Machine::new();
        let target_register = 0xAB.into();
        let source_register = 0x07.into();
        let constant = 2;
        let source_value = 40;
        let expected_value = 42;
        machine.processor.registers[source_register] = source_value;
        let machine = execute_instruction_with_machine(
            machine,
            AddTargetSourceImmediate {
                t: target_register,
                s: source_register,
                immediate: constant,
            },
        );
        assert_eq!(machine.processor.registers[source_register], source_value);
        assert_eq!(machine.processor.registers[target_register], expected_value);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn add_immediate_with_zero_flag_set() {
        let mut machine = Machine::new();
        let target_register = 0xAB.into();
        let source_register = 0x07.into();
        let constant = 0;
        let source_value = 0;
        let expected_value = 0;
        machine.processor.registers[source_register] = source_value;
        let machine = execute_instruction_with_machine(
            machine,
            AddTargetSourceImmediate {
                t: target_register,
                s: source_register,
                immediate: constant,
            },
        );
        assert_eq!(machine.processor.registers[source_register], source_value);
        assert_eq!(machine.processor.registers[target_register], expected_value);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn add_immediate_with_carry_flag_set() {
        let mut machine = Machine::new();
        let target_register = 0xAB.into();
        let source_register = 0x07.into();
        let constant = 5;
        let source_value = Word::MAX;
        let expected_value = 4;
        machine.processor.registers[source_register] = source_value;
        let machine = execute_instruction_with_machine(
            machine,
            AddTargetSourceImmediate {
                t: target_register,
                s: source_register,
                immediate: constant,
            },
        );
        assert_eq!(machine.processor.registers[source_register], source_value);
        assert_eq!(machine.processor.registers[target_register], expected_value);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn add_immediate_with_zero_and_carry_flags_set() {
        let mut machine = Machine::new();
        let target_register = 0xAB.into();
        let source_register = 0x07.into();
        let constant = 1;
        let source_value = Word::MAX;
        let expected_value = 0;
        machine.processor.registers[source_register] = source_value;
        let machine = execute_instruction_with_machine(
            machine,
            AddTargetSourceImmediate {
                t: target_register,
                s: source_register,
                immediate: constant,
            },
        );
        assert_eq!(machine.processor.registers[source_register], source_value);
        assert_eq!(machine.processor.registers[target_register], expected_value);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn subtract_immediate_with_no_flags_set() {
        let mut machine = Machine::new();
        let target_register = 0xAB.into();
        let source_register = 0x07.into();
        let constant = 2;
        let source_value = 44;
        let expected_value = 42;
        machine.processor.registers[source_register] = source_value;
        let machine = execute_instruction_with_machine(
            machine,
            SubtractTargetSourceImmediate {
                t: target_register,
                s: source_register,
                immediate: constant,
            },
        );
        assert_eq!(machine.processor.registers[source_register], source_value);
        assert_eq!(machine.processor.registers[target_register], expected_value);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn subtract_immediate_with_zero_flag_set() {
        let mut machine = Machine::new();
        let target_register = 0xAB.into();
        let source_register = 0x07.into();
        let constant = 42;
        let source_value = 42;
        let expected_value = 0;
        machine.processor.registers[source_register] = source_value;
        let machine = execute_instruction_with_machine(
            machine,
            SubtractTargetSourceImmediate {
                t: target_register,
                s: source_register,
                immediate: constant,
            },
        );
        assert_eq!(machine.processor.registers[source_register], source_value);
        assert_eq!(machine.processor.registers[target_register], expected_value);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
        assert_eq!(machine.processor.get_flag(Flag::Carry), false);
    }

    #[test]
    fn subtract_immediate_with_carry_flag_set() {
        let mut machine = Machine::new();
        let target_register = 0xAB.into();
        let source_register = 0x07.into();
        let constant = 2;
        let source_value = 1;
        let expected_value = Word::MAX;
        machine.processor.registers[source_register] = source_value;
        let machine = execute_instruction_with_machine(
            machine,
            SubtractTargetSourceImmediate {
                t: target_register,
                s: source_register,
                immediate: constant,
            },
        );
        assert_eq!(machine.processor.registers[source_register], source_value);
        assert_eq!(machine.processor.registers[target_register], expected_value);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
        assert_eq!(machine.processor.get_flag(Flag::Carry), true);
    }

    #[test]
    fn compare_lower_value_against_higher_value() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = 10;
        let rhs = 12;
        let expected = Word::MAX;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            CompareTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
    }

    #[test]
    fn compare_higher_value_against_lower_value() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = 14;
        let rhs = 12;
        let expected = 1;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            CompareTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), false);
    }

    #[test]
    fn compare_equal_values() {
        let lhs_register = 0x42.into();
        let rhs_register = 0x43.into();
        let target_register = 0x0A.into();
        let lhs = 12;
        let rhs = 12;
        let expected = 0;
        let mut machine = Machine::new();
        machine.processor.registers[lhs_register] = lhs;
        machine.processor.registers[rhs_register] = rhs;
        let mut machine = execute_instruction_with_machine(
            machine,
            CompareTargetLhsRhs {
                t: target_register,
                l: lhs_register,
                r: rhs_register,
            },
        );
        machine.make_tick();
        assert_eq!(machine.processor.registers[lhs_register], lhs);
        assert_eq!(machine.processor.registers[rhs_register], rhs);
        assert_eq!(machine.processor.registers[target_register], expected);
        assert_eq!(machine.processor.get_flag(Flag::Zero), true);
    }

    #[test]
    fn push_and_pop_stack_value() {
        let mut machine = Machine::new();
        let source_register = 0xAB.into();
        let target_register = 0x06.into();
        let data = 42;
        machine.processor.registers[source_register] = data;
        assert_eq!(
            machine.processor.get_stack_pointer(),
            Processor::STACK_START
        );
        let machine =
            execute_instruction_with_machine(machine, PushRegister { r: source_register });
        assert_eq!(
            machine.processor.get_stack_pointer(),
            Processor::STACK_START + Word::SIZE as Address
        );
        assert_eq!(machine.memory.read_data(Processor::STACK_START), data);
        let machine = execute_instruction_with_machine(machine, PopRegister { r: target_register });
        assert_eq!(
            machine.processor.get_stack_pointer(),
            Processor::STACK_START
        );
        assert_eq!(machine.processor.registers[target_register], data);
    }

    #[test]
    fn push_and_pop_multiple_stack_values() {
        let values = [1, 4, 5, 42, 2, 3];
        let mut machine = Machine::new();
        for (register, value) in (0..).map(Register).zip(values) {
            machine.processor.registers[register] = value;
            machine = execute_instruction_with_machine(machine, PushRegister { r: register });
            assert_eq!(
                machine.processor.get_stack_pointer(),
                Processor::STACK_START + (register.0 as Address + 1) * Word::SIZE as Address
            );
            assert_eq!(
                machine.memory.read_data(
                    Processor::STACK_START + register.0 as Address * Word::SIZE as Address
                ),
                value
            );
        }
        for &value in values.iter().rev() {
            let target = 0xAB.into();
            machine = execute_instruction_with_machine(machine, PopRegister { r: target });
            assert_eq!(machine.processor.registers[target], value);
        }
        assert_eq!(
            machine.processor.get_stack_pointer(),
            Processor::STACK_START
        );
    }
}

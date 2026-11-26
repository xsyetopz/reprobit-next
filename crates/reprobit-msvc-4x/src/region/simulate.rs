//! Symbolic execution of a straight run of i386 instructions. A value is a
//! term over the run's starting values. An operation the simulator does not
//! interpret is a function of its code and inputs, so the same operation on
//! the same values gives the same result. Sums are kept as sorted lists
//! with a constant, and the two operands of `add`, `imul`, `and`, `or`,
//! `xor`, and `test` are sorted, since those commute (Lean `Simulation`). A
//! load is the stores before it that may overlap its address, in order;
//! addresses that differ only by a constant are apart when the distance is
//! at least the lower access's width.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use iced_x86::{
    Code, ConstantOffsets, Encoder, Instruction, InstructionInfo, InstructionInfoFactory, Mnemonic,
    OpAccess, OpKind, Register,
};

use crate::correspond::{self, GENERAL};
use crate::schedule::Location;

/// One instruction to simulate, with the relocation targets of its fields
/// by offset within it.
pub(crate) struct Step {
    instruction: Instruction,
    symbols: BTreeMap<usize, String>,
    offsets: ConstantOffsets,
}

impl Step {
    /// `instruction`, whose fields at the offsets `symbols` names are
    /// relocated to those targets.
    pub(crate) fn new(instruction: Instruction, symbols: BTreeMap<usize, String>) -> Result<Self> {
        let mut encoder = Encoder::new(32);
        encoder.encode(&instruction, instruction.ip())?;
        let offsets = correspond::constants(&encoder.take_buffer());
        Ok(Self {
            instruction,
            symbols,
            offsets,
        })
    }

    /// The instruction.
    pub(crate) const fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    fn address(&self, state: &State) -> Term {
        let instruction = &self.instruction;
        let mut terms = Vec::new();
        if instruction.memory_base() != Register::None {
            terms.push(state.register(instruction.memory_base()));
        }
        if instruction.memory_index() != Register::None {
            let index = state.register(instruction.memory_index());
            terms.push(match instruction.memory_index_scale() {
                1 => index,
                scale => Term::Operation(Code::INVALID, scale, vec![index]),
            });
        }
        let displacement = instruction.memory_displacement32();
        match self.symbols.get(&self.offsets.displacement_offset()) {
            Some(target) if self.offsets.has_displacement() => sum(
                terms
                    .into_iter()
                    .chain([Term::Symbol(target.clone(), displacement)]),
                0,
            ),
            _ => sum(terms, displacement),
        }
    }

    fn operand(&self, state: &State, index: u32) -> Result<Term> {
        let instruction = &self.instruction;
        Ok(match instruction.op_kind(index) {
            OpKind::Register => state.register(instruction.op_register(index)),
            OpKind::Memory => state.load(self.address(state), instruction.memory_size().size()),
            _ => {
                let value = u32::try_from(instruction.immediate(index) & 0xffff_ffff)?;
                match self.symbols.get(&self.offsets.immediate_offset()) {
                    Some(target) if self.offsets.has_immediate() => {
                        Term::Symbol(target.clone(), value)
                    }
                    _ => Term::Constant(value),
                }
            }
        })
    }

    /// Whether operand `index` is a 32-bit register.
    fn wide(&self, index: u32) -> bool {
        self.instruction.op_kind(index) == OpKind::Register
            && self.instruction.op_register(index).size() == 4
    }

    /// Writes `value` to the first operand.
    fn write(&self, state: &mut State, value: Term) -> Result<()> {
        let instruction = &self.instruction;
        match instruction.op_kind(0) {
            OpKind::Register => {
                let register = instruction.op_register(0);
                let full = Location::Register(register.full_register32());
                let value = match state.register(register) {
                    Term::Part(part, old) => {
                        Term::Operation(Code::INVALID, 0x400 + u32::from(part), vec![*old, value])
                    }
                    _ => value,
                };
                state.values.insert(full, value);
            }
            OpKind::Memory => {
                let address = self.address(state);
                state
                    .stores
                    .push((address, instruction.memory_size().size(), value));
            }
            _ => bail!("its first operand is not a place"),
        }
        Ok(())
    }
}

/// A value over the run's starting state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Term {
    Start(Location),
    Constant(u32),
    /// A relocated field: its target plus the addend it holds.
    Symbol(String, u32),
    /// The terms' wrapping sum plus a constant, with the terms sorted.
    Sum(Vec<Self>, u32),
    /// The low byte, high byte, or low word of a register's value.
    Part(u8, Box<Self>),
    /// Output `tag` of an operation the simulator does not interpret.
    Operation(Code, u32, Vec<Self>),
    /// What `width` bytes at an address hold after the stores that may
    /// overlap them.
    Load(Box<Self>, usize, Vec<Store>),
}

/// An address, a width, and the value stored there.
type Store = (Term, usize, Term);

fn sum(terms: impl IntoIterator<Item = Term>, constant: u32) -> Term {
    let (mut flat, mut total) = (Vec::new(), constant);
    for term in terms {
        match term {
            Term::Sum(inner, more) => {
                flat.extend(inner);
                total = total.wrapping_add(more);
            }
            Term::Constant(value) => total = total.wrapping_add(value),
            other => flat.push(other),
        }
    }
    flat.sort();
    if flat.is_empty() {
        Term::Constant(total)
    } else {
        Term::Sum(flat, total)
    }
}

/// Whether two accesses cannot overlap: their addresses share every term
/// and differ by a constant at least the lower one's width.
fn apart(one: &Store, address: &Term, width: usize) -> bool {
    let split = |address: &Term| match address {
        Term::Sum(terms, constant) => (terms.clone(), *constant),
        Term::Constant(value) => (Vec::new(), *value),
        other => (vec![other.clone()], 0),
    };
    let ((terms, low), (other, high)) = (split(&one.0), split(address));
    let distance = i64::from(high.wrapping_sub(low).cast_signed());
    let wide = |width: usize| i64::try_from(width).unwrap_or(i64::MAX);
    terms == other && (distance >= wide(one.1) || -distance >= wide(width))
}

/// The machine state after some instructions.
#[derive(Default)]
pub(crate) struct State {
    values: BTreeMap<Location, Term>,
    stores: Vec<Store>,
}

impl State {
    /// What `location` holds.
    pub(crate) fn value(&self, location: Location) -> Term {
        self.values
            .get(&location)
            .cloned()
            .unwrap_or(Term::Start(location))
    }

    fn load(&self, address: Term, width: usize) -> Term {
        let mut overlapping = Vec::new();
        for store in self.stores.iter().rev() {
            if store.0 == address && store.1 == width && overlapping.is_empty() {
                return store.2.clone();
            }
            if !apart(store, &address, width) {
                overlapping.push(store.clone());
            }
        }
        overlapping.reverse();
        Term::Load(Box::new(address), width, overlapping)
    }

    /// Each stored place and what loading it gives.
    pub(crate) fn memory(&self) -> BTreeMap<(Term, usize), Term> {
        self.stores
            .iter()
            .map(|(address, width, _)| {
                (
                    (address.clone(), *width),
                    self.load(address.clone(), *width),
                )
            })
            .collect()
    }

    fn register(&self, register: Register) -> Term {
        let full = self.value(Location::Register(register.full_register32()));
        if register.full_register32() == register {
            return full;
        }
        let high = [Register::AH, Register::CH, Register::DH, Register::BH];
        let part = match register.size() {
            2 => 2,
            _ if high.contains(&register) => 1,
            _ => 0,
        };
        Term::Part(part, Box::new(full))
    }
}

/// The state after `run`.
pub(crate) fn simulate<'a>(run: impl IntoIterator<Item = &'a Step>) -> Result<State> {
    let mut state = State::default();
    let mut info = InstructionInfoFactory::new();
    for step in run {
        execute(&mut state, &mut info, step).with_context(|| {
            format!(
                "cannot simulate the instruction at {:#x}",
                step.instruction.ip()
            )
        })?;
    }
    Ok(state)
}

fn execute(state: &mut State, info: &mut InstructionInfoFactory, step: &Step) -> Result<()> {
    let instruction = &step.instruction;
    let esp = Location::Register(Register::ESP);
    match instruction.mnemonic() {
        Mnemonic::Nop => Ok(()),
        Mnemonic::Push if instruction.op_count() == 1 => {
            let value = step.operand(state, 0)?;
            let top = sum([state.value(esp)], 4u32.wrapping_neg());
            state.values.insert(esp, top.clone());
            state.stores.push((top, 4, value));
            Ok(())
        }
        Mnemonic::Pop if step.wide(0) => {
            let top = state.value(esp);
            let value = state.load(top.clone(), 4);
            state.values.insert(esp, sum([top], 4));
            step.write(state, value)
        }
        Mnemonic::Mov
            if step.wide(0)
                || (instruction.op_kind(0) == OpKind::Memory
                    && instruction.memory_size().size() == 4) =>
        {
            let value = step.operand(state, 1)?;
            step.write(state, value)
        }
        Mnemonic::Lea if step.wide(0) => {
            let address = step.address(state);
            step.write(state, address)
        }
        _ => operate(state, info.info(instruction), step),
    }
}

/// Whether an instruction reads or writes the x87 state.
fn float(instruction: &Instruction, used: &InstructionInfo) -> bool {
    format!("{:?}", instruction.mnemonic()).starts_with('F')
        || used
            .used_registers()
            .iter()
            .any(|register| register.register().is_st())
}

/// Any other instruction: each location it writes gets a term of its code
/// and inputs, but `add` to a 32-bit register, which `sum` keeps.
fn operate(state: &mut State, used: &InstructionInfo, step: &Step) -> Result<()> {
    let instruction = &step.instruction;
    let explicit_memory =
        (0..instruction.op_count()).any(|index| instruction.op_kind(index) == OpKind::Memory);
    ensure!(
        used.used_memory().len() <= usize::from(explicit_memory),
        "it reaches memory its operands do not name"
    );
    let (inputs, commutes) = inputs(state, used, step)?;
    let written = instruction.op_count() > 0 && writes(used.op_access(0));
    let destination = (written && instruction.op_kind(0) == OpKind::Register)
        .then(|| instruction.op_register(0).full_register32());
    // `iced-x86` reads nothing for `xor r, r` and `sub r, r`, so whichever
    // register they zero, their outputs are the same terms.
    let output = |tag: u32| Term::Operation(instruction.code(), tag, inputs.clone());
    let implicit: BTreeSet<Register> = used
        .used_registers()
        .iter()
        .filter(|register| writes(register.access()))
        .map(|register| register.register().full_register32())
        .filter(|full| GENERAL.contains(full) && destination != Some(*full))
        .collect();
    let result = match instruction.mnemonic() {
        Mnemonic::Add if commutes && step.wide(0) => sum(inputs[..2].iter().cloned(), 0),
        _ => output(0),
    };
    for flag in correspond::flags(instruction.rflags_modified()) {
        if let Location::Flag(bit) = flag {
            state.values.insert(flag, output(0x100 + bit));
        }
    }
    for register in implicit {
        let tag = 0x200 + u32::from(register as u8);
        state
            .values
            .insert(Location::Register(register), output(tag));
    }
    if float(instruction, used) {
        state.values.insert(Location::Float, output(0x300));
    }
    if written {
        step.write(state, result)?;
    }
    Ok(())
}

/// What an instruction reads, in operand order, then its implicit
/// registers, flags, and x87 state; and whether its two operands commute,
/// in which case they come sorted.
fn inputs(state: &State, used: &InstructionInfo, step: &Step) -> Result<(Vec<Term>, bool)> {
    let instruction = &step.instruction;
    let mut inputs = Vec::new();
    for index in 0..instruction.op_count() {
        if reads(used.op_access(index)) {
            inputs.push(step.operand(state, index)?);
        } else if instruction.op_kind(index) == OpKind::Register
            && instruction.op_register(index).size() < 4
        {
            let full = instruction.op_register(index).full_register32();
            inputs.push(state.value(Location::Register(full)));
        }
    }
    let commutes = matches!(
        instruction.mnemonic(),
        Mnemonic::Add
            | Mnemonic::Imul
            | Mnemonic::And
            | Mnemonic::Or
            | Mnemonic::Xor
            | Mnemonic::Test
    ) && instruction.op_count() == 2
        && inputs.len() == 2;
    if commutes {
        inputs.sort();
    }
    let named = correspond::explicit(instruction);
    for register in used.used_registers() {
        let full = register.register().full_register32();
        if GENERAL.contains(&full) && !named.contains(&full) && reads(register.access()) {
            inputs.push(state.value(Location::Register(full)));
        }
    }
    inputs.extend(correspond::flags(instruction.rflags_read()).map(|flag| state.value(flag)));
    if float(instruction, used) {
        inputs.push(state.value(Location::Float));
    }
    Ok((inputs, commutes))
}

fn reads(access: OpAccess) -> bool {
    matches!(
        access,
        OpAccess::Read
            | OpAccess::ReadWrite
            | OpAccess::CondRead
            | OpAccess::ReadCondWrite
            | OpAccess::CondWrite
    )
}

fn writes(access: OpAccess) -> bool {
    matches!(
        access,
        OpAccess::Write | OpAccess::ReadWrite | OpAccess::CondWrite | OpAccess::ReadCondWrite
    )
}

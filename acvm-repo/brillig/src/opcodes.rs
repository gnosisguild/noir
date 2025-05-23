use crate::black_box::BlackBoxOp;
use acir_field::AcirField;
use serde::{Deserialize, Serialize};

pub type Label = usize;

/// Represents an address in the VM's memory.
/// Supports both direct and relative addressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub enum MemoryAddress {
    /// Specifies an exact index in the VM's memory
    Direct(usize),
    /// Specifies an index relative to the stack pointer.
    ///
    /// It is resolved as the current stack pointer plus the offset stored here.
    Relative(usize),
}

impl MemoryAddress {
    pub fn direct(address: usize) -> Self {
        MemoryAddress::Direct(address)
    }
    pub fn relative(offset: usize) -> Self {
        MemoryAddress::Relative(offset)
    }

    pub fn unwrap_direct(self) -> usize {
        match self {
            MemoryAddress::Direct(address) => address,
            MemoryAddress::Relative(_) => panic!("Expected direct memory address"),
        }
    }

    pub fn unwrap_relative(self) -> usize {
        match self {
            MemoryAddress::Direct(_) => panic!("Expected relative memory address"),
            MemoryAddress::Relative(offset) => offset,
        }
    }

    pub fn to_usize(self) -> usize {
        match self {
            MemoryAddress::Direct(address) => address,
            MemoryAddress::Relative(offset) => offset,
        }
    }

    pub fn is_relative(&self) -> bool {
        match self {
            MemoryAddress::Relative(_) => true,
            MemoryAddress::Direct(_) => false,
        }
    }

    pub fn offset(&self, amount: usize) -> Self {
        match self {
            MemoryAddress::Direct(address) => MemoryAddress::Direct(address + amount),
            MemoryAddress::Relative(offset) => MemoryAddress::Relative(offset + amount),
        }
    }
}

/// Describes the memory layout for an array/vector element
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum HeapValueType {
    // A single field element is enough to represent the value with a given bit size
    Simple(BitSize),
    // The value read should be interpreted as a pointer to a heap array, which
    // consists of a pointer to a slice of memory of size elements, and a
    // reference count
    Array { value_types: Vec<HeapValueType>, size: usize },
    // The value read should be interpreted as a pointer to a heap vector, which
    // consists of a pointer to a slice of memory, a number of elements in that
    // slice, and a reference count
    Vector { value_types: Vec<HeapValueType> },
}

impl HeapValueType {
    pub fn all_simple(types: &[HeapValueType]) -> bool {
        types.iter().all(|typ| matches!(typ, HeapValueType::Simple(_)))
    }

    pub fn field() -> HeapValueType {
        HeapValueType::Simple(BitSize::Field)
    }
}

/// A fixed-sized array starting from a Brillig memory location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub struct HeapArray {
    pub pointer: MemoryAddress,
    pub size: usize,
}

impl Default for HeapArray {
    fn default() -> Self {
        Self { pointer: MemoryAddress::direct(0), size: 0 }
    }
}

/// A memory-sized vector passed starting from a Brillig memory location and with a memory-held size
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub struct HeapVector {
    pub pointer: MemoryAddress,
    pub size: MemoryAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub enum IntegerBitSize {
    U1,
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl From<IntegerBitSize> for u32 {
    fn from(bit_size: IntegerBitSize) -> u32 {
        match bit_size {
            IntegerBitSize::U1 => 1,
            IntegerBitSize::U8 => 8,
            IntegerBitSize::U16 => 16,
            IntegerBitSize::U32 => 32,
            IntegerBitSize::U64 => 64,
            IntegerBitSize::U128 => 128,
        }
    }
}

impl TryFrom<u32> for IntegerBitSize {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(IntegerBitSize::U1),
            8 => Ok(IntegerBitSize::U8),
            16 => Ok(IntegerBitSize::U16),
            32 => Ok(IntegerBitSize::U32),
            64 => Ok(IntegerBitSize::U64),
            128 => Ok(IntegerBitSize::U128),
            _ => Err("Invalid bit size"),
        }
    }
}

impl std::fmt::Display for IntegerBitSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IntegerBitSize::U1 => write!(f, "bool"),
            IntegerBitSize::U8 => write!(f, "u8"),
            IntegerBitSize::U16 => write!(f, "u16"),
            IntegerBitSize::U32 => write!(f, "u32"),
            IntegerBitSize::U64 => write!(f, "u64"),
            IntegerBitSize::U128 => write!(f, "u128"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub enum BitSize {
    Field,
    Integer(IntegerBitSize),
}

impl BitSize {
    pub fn to_u32<F: AcirField>(self) -> u32 {
        match self {
            BitSize::Field => F::max_num_bits(),
            BitSize::Integer(bit_size) => bit_size.into(),
        }
    }

    pub fn try_from_u32<F: AcirField>(value: u32) -> Result<Self, &'static str> {
        if value == F::max_num_bits() {
            Ok(BitSize::Field)
        } else {
            Ok(BitSize::Integer(IntegerBitSize::try_from(value)?))
        }
    }
}

/// Lays out various ways an external foreign call's input and output data may be interpreted inside Brillig.
/// This data can either be an individual value or memory.
///
/// While we are usually agnostic to how memory is passed within Brillig,
/// this needs to be encoded somehow when dealing with an external system.
/// For simplicity, the extra type information is given right in the ForeignCall instructions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub enum ValueOrArray {
    /// A single value passed to or from an external call
    /// It is an 'immediate' value - used without dereferencing.
    /// For a foreign call input, the value is read directly from memory.
    /// For a foreign call output, the value is written directly to memory.
    MemoryAddress(MemoryAddress),
    /// An array passed to or from an external call
    /// In the case of a foreign call input, the array is read from this Brillig memory location + usize more cells.
    /// In the case of a foreign call output, the array is written to this Brillig memory location with the usize being here just as a sanity check for the size write.
    HeapArray(HeapArray),
    /// A vector passed to or from an external call
    /// In the case of a foreign call input, the vector is read from this Brillig memory location + as many cells as the 2nd address indicates.
    /// In the case of a foreign call output, the vector is written to this Brillig memory location and as 'size' cells, with size being stored in the second address.
    HeapVector(HeapVector),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub enum BrilligOpcode<F> {
    /// Takes the fields in addresses `lhs` and `rhs`
    /// Performs the specified binary operation
    /// and stores the value in the `destination` address.
    BinaryFieldOp {
        destination: MemoryAddress,
        op: BinaryFieldOp,
        lhs: MemoryAddress,
        rhs: MemoryAddress,
    },
    /// Takes the `bit_size` size integers in addresses `lhs` and `rhs`
    /// Performs the specified binary operation
    /// and stores the value in the `destination` address.
    BinaryIntOp {
        destination: MemoryAddress,
        op: BinaryIntOp,
        bit_size: IntegerBitSize,
        lhs: MemoryAddress,
        rhs: MemoryAddress,
    },
    Not {
        destination: MemoryAddress,
        source: MemoryAddress,
        bit_size: IntegerBitSize,
    },
    Cast {
        destination: MemoryAddress,
        source: MemoryAddress,
        bit_size: BitSize,
    },
    /// Sets the program counter to the value of `location` if
    /// the value at the `condition` address is zero.
    JumpIfNot {
        condition: MemoryAddress,
        location: Label,
    },
    /// Sets the program counter to the value of `location`
    /// If the value at `condition` is non-zero
    JumpIf {
        condition: MemoryAddress,
        location: Label,
    },
    /// Sets the program counter to the value of `location`.
    Jump {
        location: Label,
    },
    /// Copies calldata after the offset to the specified address and length
    CalldataCopy {
        destination_address: MemoryAddress,
        size_address: MemoryAddress,
        offset_address: MemoryAddress,
    },
    /// We don't support dynamic jumps or calls
    /// See <https://github.com/ethereum/aleth/issues/3404> for reasoning
    ///
    /// Pushes the current program counter to the call stack as to set a return location.
    /// Sets the program counter to the value of `location`.
    Call {
        location: Label,
    },
    /// Stores a constant `value` with a `bit_size` in the `destination` address.
    Const {
        destination: MemoryAddress,
        bit_size: BitSize,
        value: F,
    },
    /// Reads the address from `destination_pointer`, then stores a constant `value` with a `bit_size` at that address.
    IndirectConst {
        destination_pointer: MemoryAddress,
        bit_size: BitSize,
        value: F,
    },
    /// Pops the top element from the call stack, which represents the return location,
    /// and sets the program counter to that value. This operation is used to return
    /// from a function call.
    Return,
    /// Used to get data from an outside source.
    /// Also referred to as an Oracle. However, we don't use that name as
    /// this is intended for things like state tree reads, and shouldn't be confused
    /// with e.g. blockchain price oracles.
    ForeignCall {
        /// Interpreted by caller context, ie this will have different meanings depending on
        /// who the caller is.
        function: String,
        /// Destination addresses (may be single values or memory pointers).
        destinations: Vec<ValueOrArray>,
        /// Destination value types
        destination_value_types: Vec<HeapValueType>,
        /// Input addresses (may be single values or memory pointers).
        inputs: Vec<ValueOrArray>,
        /// Input value types (for heap allocated structures indicates how to
        /// retrieve the elements)
        input_value_types: Vec<HeapValueType>,
    },
    /// Moves the content in the `source` address to the `destination` address.
    Mov {
        destination: MemoryAddress,
        source: MemoryAddress,
    },
    /// destination = condition > 0 ? source_a : source_b
    ConditionalMov {
        destination: MemoryAddress,
        source_a: MemoryAddress,
        source_b: MemoryAddress,
        condition: MemoryAddress,
    },
    /// Reads the `source_pointer` to obtain a memory address, then retrieves the data
    /// stored at that address and writes it to the `destination` address.
    Load {
        destination: MemoryAddress,
        source_pointer: MemoryAddress,
    },
    /// Reads the `destination_pointer` to obtain a memory address, then stores the value
    /// from the `source` address at that location.
    Store {
        destination_pointer: MemoryAddress,
        source: MemoryAddress,
    },
    /// Native functions in the VM.
    /// These are equivalent to the black box functions in ACIR.
    BlackBox(BlackBoxOp),
    /// Used to denote execution failure, halting the VM and returning data specified by a dynamically-sized vector.
    Trap {
        revert_data: HeapVector,
    },
    /// Halts execution and returns data specified by a dynamically-sized vector.
    Stop {
        return_data: HeapVector,
    },
}

/// Binary fixed-length field expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub enum BinaryFieldOp {
    Add,
    Sub,
    Mul,
    /// Field division
    Div,
    /// Integer division
    IntegerDiv,
    /// (==) equal
    Equals,
    /// (<) Field less than
    LessThan,
    /// (<=) field less or equal
    LessThanEquals,
}

/// Binary fixed-length integer expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[cfg_attr(feature = "arb", derive(proptest_derive::Arbitrary))]
pub enum BinaryIntOp {
    Add,
    Sub,
    Mul,
    Div,
    /// (==) equal
    Equals,
    /// (<) Field less than
    LessThan,
    /// (<=) field less or equal
    LessThanEquals,
    /// (&) Bitwise AND
    And,
    /// (|) Bitwise OR
    Or,
    /// (^) Bitwise XOR
    Xor,
    /// (<<) Shift left
    Shl,
    /// (>>) Shift right
    Shr,
}

#[cfg(feature = "arb")]
mod tests {
    use proptest::arbitrary::Arbitrary;
    use proptest::prelude::*;

    use super::{BitSize, HeapValueType};

    // Need to define recursive strategy for `HeapValueType`
    impl Arbitrary for HeapValueType {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            let leaf = any::<BitSize>().prop_map(HeapValueType::Simple);
            leaf.prop_recursive(2, 3, 2, |inner| {
                prop_oneof![
                    (prop::collection::vec(inner.clone(), 1..3), any::<usize>()).prop_map(
                        |(value_types, size)| { HeapValueType::Array { value_types, size } }
                    ),
                    (prop::collection::vec(inner.clone(), 1..3))
                        .prop_map(|value_types| { HeapValueType::Vector { value_types } }),
                ]
            })
            .boxed()
        }
    }
}

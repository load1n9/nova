use crate::value::Value;

use super::{
    array::ArrayHeapData, bigint::BigIntHeapData, date::DateHeapData, error::ErrorHeapData,
    function::FunctionHeapData, number::NumberHeapData, object::ObjectHeapData,
    regexp::RegExpHeapData, string::StringHeapData, symbol::SymbolHeapData,
};
use core::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::{marker::PhantomData, mem::size_of, num::NonZeroU32};

/// A struct containing a non-zero index into an array or
/// vector of `T`s. Due to the non-zero value, the offset
/// in the vector is offset by one.
///
/// This index implies a tracing reference count from this
/// struct to T at the given index.
pub(crate) struct BaseIndex<T: ?Sized>(NonZeroU32, PhantomData<T>);

const _INDEX_SIZE_IS_U32: () = assert!(size_of::<BaseIndex<()>>() == size_of::<u32>());
const _OPTION_INDEX_SIZE_IS_U32: () =
    assert!(size_of::<Option<BaseIndex<()>>>() == size_of::<u32>());

impl<T: ?Sized> Debug for BaseIndex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(self.0.get() != 0);
        (&self.0.get() - 1).fmt(f)
    }
}

impl<T: ?Sized> Clone for BaseIndex<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for BaseIndex<T> {}

impl<T: ?Sized> PartialEq for BaseIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl<T: ?Sized> Eq for BaseIndex<T> {}

impl<T: ?Sized> PartialOrd for BaseIndex<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: ?Sized> Ord for BaseIndex<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized> Hash for BaseIndex<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: ?Sized> BaseIndex<T> {
    pub const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }

    pub const fn into_u32_index(self) -> u32 {
        self.0.get() as u32 - 1
    }

    pub const fn into_usize(self) -> usize {
        self.0.get() as usize
    }

    pub const fn into_u32(self) -> u32 {
        self.0.get() as u32
    }

    pub const fn from_index(value: usize) -> Self {
        let value = value as u32;
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(
            unsafe { NonZeroU32::new_unchecked(value as u32 + 1) },
            PhantomData,
        )
    }

    pub const fn from_u32_index(value: u32) -> Self {
        let value = value as u32;
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
    }

    pub const fn from_usize(value: usize) -> Self {
        let value = value as u32;
        assert!(value != 0);
        // SAFETY: Number is not zero.
        // This check is done manually to allow const context.
        Self(
            unsafe { NonZeroU32::new_unchecked(value as u32) },
            PhantomData,
        )
    }

    pub const fn from_u32(value: u32) -> Self {
        let value = value as u32;
        assert!(value != 0);
        // SAFETY: Number is not zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value) }, PhantomData)
    }

    pub fn last<U: Sized>(vec: &Vec<Option<U>>) -> Self {
        assert!(vec.len() > 0);
        Self::from_index(vec.len())
    }
}

pub(crate) type ArrayIndex = BaseIndex<ArrayHeapData>;
pub(crate) type BigIntIndex = BaseIndex<BigIntHeapData>;
pub(crate) type DateIndex = BaseIndex<DateHeapData>;
pub(crate) type ErrorIndex = BaseIndex<ErrorHeapData>;
pub(crate) type FunctionIndex = BaseIndex<FunctionHeapData>;
pub(crate) type NumberIndex = BaseIndex<NumberHeapData>;
pub(crate) type ObjectIndex = BaseIndex<ObjectHeapData>;
pub(crate) type RegExpIndex = BaseIndex<RegExpHeapData>;
pub(crate) type StringIndex = BaseIndex<StringHeapData>;
pub(crate) type SymbolIndex = BaseIndex<SymbolHeapData>;
pub(crate) type ElementIndex = BaseIndex<[Option<Value>]>;

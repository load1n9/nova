// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{
    BigInt, BigIntHeapData, IntoValue, Number, Numeric, OrdinaryObject, Primitive, String,
    StringHeapData, Symbol, bigint::HeapBigInt, number::HeapNumber, string::HeapString,
};
#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::Date;
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::RegExp;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "set")]
use crate::ecmascript::builtins::{
    keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIterator, set::Set,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{weak_map::WeakMap, weak_ref::WeakRef, weak_set::WeakSet};
use crate::{
    SmallInteger, SmallString,
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int, to_int16, to_int32, to_number, to_numeric, to_string, to_uint16, to_uint32,
            try_to_string,
        },
        builtins::{
            Array, BuiltinConstructorFunction, BuiltinFunction, ECMAScriptFunction,
            async_generator_objects::AsyncGenerator,
            bound_function::BoundFunction,
            control_abstraction_objects::{
                generator_objects::Generator,
                promise_objects::promise_abstract_operations::promise_resolving_functions::BuiltinPromiseResolvingFunction,
            },
            embedder_object::EmbedderObject,
            error::Error,
            finalization_registry::FinalizationRegistry,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIterator,
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIterator,
            map::Map,
            module::Module,
            primitive_objects::PrimitiveObject,
            promise::Promise,
            proxy::Proxy,
            text_processing::string_objects::string_iterator_objects::StringIterator,
        },
        execution::{Agent, JsResult},
        types::{BUILTIN_STRING_MEMORY, Object},
    },
    engine::{
        Scoped, TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        small_bigint::SmallBigInt,
        small_f64::SmallF64,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};
#[cfg(feature = "array-buffer")]
use crate::{
    ecmascript::builtins::{ArrayBuffer, data_view::DataView},
    heap::indexes::TypedArrayIndex,
};

use core::{
    hash::{Hash, Hasher},
    mem::size_of,
    ops::Index,
};

/// ### [6.1 ECMAScript Language Types](https://tc39.es/ecma262/#sec-ecmascript-language-types)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(u8)]
pub enum Value<'a> {
    /// ### [6.1.1 The Undefined Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-undefined-type)
    #[default]
    Undefined = 1,

    /// ### [6.1.2 The Null Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-null-type)
    Null,

    /// ### [6.1.3 The Boolean Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-boolean-type)
    Boolean(bool),

    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// UTF-8 string on the heap. Accessing the data must be done through the
    /// Agent. ECMAScript specification compliant UTF-16 indexing is
    /// implemented through an index mapping.
    String(HeapString<'a>),
    /// ### [6.1.4 The String Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-string-type)
    ///
    /// 7-byte UTF-8 string on the stack. End of the string is determined by
    /// the first 0xFF byte in the data. UTF-16 indexing is calculated on
    /// demand from the data.
    SmallString(SmallString),

    /// ### [6.1.5 The Symbol Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-symbol-type)
    Symbol(Symbol<'a>),

    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// f64 on the heap. Accessing the data must be done through the Agent.
    Number(HeapNumber<'a>),
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 53-bit signed integer on the stack.
    Integer(SmallInteger),
    /// ### [6.1.6.1 The Number Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-number-type)
    ///
    /// 56-bit f64 on the stack. The missing byte is a zero least significant
    /// byte.
    SmallF64(SmallF64),

    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// Unlimited size integer data on the heap. Accessing the data must be
    /// done through the Agent.
    BigInt(HeapBigInt<'a>),
    /// ### [6.1.6.2 The BigInt Type](https://tc39.es/ecma262/#sec-ecmascript-language-types-bigint-type)
    ///
    /// 56-bit signed integer on the stack.
    SmallBigInt(SmallBigInt),

    /// ### [6.1.7 The Object Type](https://tc39.es/ecma262/#sec-object-type)
    Object(OrdinaryObject<'a>),

    // Functions
    BoundFunction(BoundFunction<'a>),
    BuiltinFunction(BuiltinFunction<'a>),
    ECMAScriptFunction(ECMAScriptFunction<'a>),
    // TODO: Figure out if all the special function types are wanted or if we'd
    // prefer to just keep them as internal variants of the three above ones.
    BuiltinGeneratorFunction,
    /// Default class constructor created in step 14 of
    /// [ClassDefinitionEvaluation](https://tc39.es/ecma262/#sec-runtime-semantics-classdefinitionevaluation).
    BuiltinConstructorFunction(BuiltinConstructorFunction<'a>),
    BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction<'a>),
    BuiltinPromiseCollectorFunction,
    BuiltinProxyRevokerFunction,

    // Boolean, Number, String, Symbol, BigInt objects
    PrimitiveObject(PrimitiveObject<'a>),

    // Well-known object types
    // Roughly corresponding to 6.1.7.4 Well-Known Intrinsic Objects
    // https://tc39.es/ecma262/#sec-well-known-intrinsic-objects
    // and 18 ECMAScript Standard Built-in Objects
    // https://tc39.es/ecma262/#sec-ecmascript-standard-built-in-objects
    /// ### [10.4.4 Arguments Exotic Objects](https://tc39.es/ecma262/#sec-arguments-exotic-objects)
    ///
    /// An unmapped arguments object is an ordinary object with an additional
    /// internal slot \[\[ParameterMap]] whose value is always **undefined**.
    Arguments(OrdinaryObject<'a>),
    // TODO: MappedArguments(MappedArgumentsObject),
    Array(Array<'a>),
    #[cfg(feature = "array-buffer")]
    ArrayBuffer(ArrayBuffer<'a>),
    #[cfg(feature = "array-buffer")]
    DataView(DataView<'a>),
    #[cfg(feature = "date")]
    Date(Date<'a>),
    Error(Error<'a>),
    FinalizationRegistry(FinalizationRegistry<'a>),
    Map(Map<'a>),
    Promise(Promise<'a>),
    Proxy(Proxy<'a>),
    #[cfg(feature = "regexp")]
    RegExp(RegExp<'a>),
    #[cfg(feature = "set")]
    Set(Set<'a>),
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'a>),
    #[cfg(feature = "weak-refs")]
    WeakMap(WeakMap<'a>),
    #[cfg(feature = "weak-refs")]
    WeakRef(WeakRef<'a>),
    #[cfg(feature = "weak-refs")]
    WeakSet(WeakSet<'a>),

    // TypedArrays
    #[cfg(feature = "array-buffer")]
    Int8Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    Uint8Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    Uint8ClampedArray(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    Int16Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    Uint16Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    Int32Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    Uint32Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    BigInt64Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    BigUint64Array(TypedArrayIndex<'a>),
    #[cfg(feature = "proposal-float16array")]
    Float16Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    Float32Array(TypedArrayIndex<'a>),
    #[cfg(feature = "array-buffer")]
    Float64Array(TypedArrayIndex<'a>),

    // Iterator objects
    // TODO: Figure out if these are needed at all.
    AsyncFromSyncIterator,
    AsyncGenerator(AsyncGenerator<'a>),
    ArrayIterator(ArrayIterator<'a>),
    #[cfg(feature = "set")]
    SetIterator(SetIterator<'a>),
    MapIterator(MapIterator<'a>),
    StringIterator(StringIterator<'a>),
    Generator(Generator<'a>),

    // ECMAScript Module
    Module(Module<'a>),

    // Embedder objects
    EmbedderObject(EmbedderObject<'a>) = 0x7f,
}

/// We want to guarantee that all handles to JS values are register sized. This
/// assert must never be removed or broken.
const _VALUE_SIZE_IS_WORD: () = assert!(size_of::<Value>() == size_of::<usize>());
/// We may also want to keep Option<Value> register sized so that eg. holes in
/// arrays do not start requiring extra bookkeeping.
const _OPTIONAL_VALUE_SIZE_IS_WORD: () = assert!(size_of::<Option<Value>>() == size_of::<usize>());

#[derive(Debug, Clone, Copy)]
pub enum PreferredType {
    String,
    Number,
}
const fn value_discriminant(value: Value) -> u8 {
    // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
    // between `repr(C)` structs, each of which has the `u8` discriminant as its first
    // field, so we can read the discriminant without offsetting the pointer.
    unsafe { *(&value as *const Value).cast::<u8>() }
}

pub(crate) const UNDEFINED_DISCRIMINANT: u8 = value_discriminant(Value::Undefined);
pub(crate) const NULL_DISCRIMINANT: u8 = value_discriminant(Value::Null);
pub(crate) const BOOLEAN_DISCRIMINANT: u8 = value_discriminant(Value::Boolean(true));
pub(crate) const STRING_DISCRIMINANT: u8 = value_discriminant(Value::String(HeapString::_def()));
pub(crate) const SMALL_STRING_DISCRIMINANT: u8 =
    value_discriminant(Value::SmallString(SmallString::EMPTY));
pub(crate) const SYMBOL_DISCRIMINANT: u8 = value_discriminant(Value::Symbol(Symbol::_def()));
pub(crate) const NUMBER_DISCRIMINANT: u8 = value_discriminant(Value::Number(HeapNumber::_def()));
pub(crate) const INTEGER_DISCRIMINANT: u8 =
    value_discriminant(Value::Integer(SmallInteger::zero()));
pub(crate) const FLOAT_DISCRIMINANT: u8 = value_discriminant(Value::SmallF64(SmallF64::_def()));
pub(crate) const BIGINT_DISCRIMINANT: u8 = value_discriminant(Value::BigInt(HeapBigInt::_def()));
pub(crate) const SMALL_BIGINT_DISCRIMINANT: u8 =
    value_discriminant(Value::SmallBigInt(SmallBigInt::zero()));
pub(crate) const OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::Object(OrdinaryObject::_def()));
pub(crate) const ARRAY_DISCRIMINANT: u8 = value_discriminant(Value::Array(Array::_def()));
#[cfg(feature = "array-buffer")]
pub(crate) const ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::ArrayBuffer(ArrayBuffer::_def()));
#[cfg(feature = "date")]
pub(crate) const DATE_DISCRIMINANT: u8 = value_discriminant(Value::Date(Date::_def()));
pub(crate) const ERROR_DISCRIMINANT: u8 = value_discriminant(Value::Error(Error::_def()));
pub(crate) const BUILTIN_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinFunction(BuiltinFunction::_def()));
pub(crate) const ECMASCRIPT_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::ECMAScriptFunction(ECMAScriptFunction::_def()));
pub(crate) const BOUND_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BoundFunction(BoundFunction::_def()));
#[cfg(feature = "regexp")]
pub(crate) const REGEXP_DISCRIMINANT: u8 = value_discriminant(Value::RegExp(RegExp::_def()));

pub(crate) const BUILTIN_GENERATOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinGeneratorFunction);
pub(crate) const BUILTIN_CONSTRUCTOR_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::BuiltinConstructorFunction(BuiltinConstructorFunction::_def()),
);
pub(crate) const BUILTIN_PROMISE_RESOLVING_FUNCTION_DISCRIMINANT: u8 = value_discriminant(
    Value::BuiltinPromiseResolvingFunction(BuiltinPromiseResolvingFunction::_def()),
);
pub(crate) const BUILTIN_PROMISE_COLLECTOR_FUNCTION_DISCRIMINANT: u8 =
    value_discriminant(Value::BuiltinPromiseCollectorFunction);
pub(crate) const BUILTIN_PROXY_REVOKER_FUNCTION: u8 =
    value_discriminant(Value::BuiltinProxyRevokerFunction);
pub(crate) const PRIMITIVE_OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::PrimitiveObject(PrimitiveObject::_def()));
pub(crate) const ARGUMENTS_DISCRIMINANT: u8 =
    value_discriminant(Value::Arguments(OrdinaryObject::_def()));
#[cfg(feature = "array-buffer")]
pub(crate) const DATA_VIEW_DISCRIMINANT: u8 = value_discriminant(Value::DataView(DataView::_def()));
pub(crate) const FINALIZATION_REGISTRY_DISCRIMINANT: u8 =
    value_discriminant(Value::FinalizationRegistry(FinalizationRegistry::_def()));
pub(crate) const MAP_DISCRIMINANT: u8 = value_discriminant(Value::Map(Map::_def()));
pub(crate) const PROMISE_DISCRIMINANT: u8 = value_discriminant(Value::Promise(Promise::_def()));
pub(crate) const PROXY_DISCRIMINANT: u8 = value_discriminant(Value::Proxy(Proxy::_def()));
#[cfg(feature = "set")]
pub(crate) const SET_DISCRIMINANT: u8 = value_discriminant(Value::Set(Set::_def()));
#[cfg(feature = "shared-array-buffer")]
pub(crate) const SHARED_ARRAY_BUFFER_DISCRIMINANT: u8 =
    value_discriminant(Value::SharedArrayBuffer(SharedArrayBuffer::_def()));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_MAP_DISCRIMINANT: u8 = value_discriminant(Value::WeakMap(WeakMap::_def()));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_REF_DISCRIMINANT: u8 = value_discriminant(Value::WeakRef(WeakRef::_def()));
#[cfg(feature = "weak-refs")]
pub(crate) const WEAK_SET_DISCRIMINANT: u8 = value_discriminant(Value::WeakSet(WeakSet::_def()));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int8Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_8_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint8Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_8_CLAMPED_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint8ClampedArray(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int16Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint16Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const INT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Int32Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const UINT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Uint32Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const BIGINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::BigInt64Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const BIGUINT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::BigUint64Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "proposal-float16array")]
pub(crate) const FLOAT_16_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float16Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const FLOAT_32_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float32Array(TypedArrayIndex::from_u32_index(0)));
#[cfg(feature = "array-buffer")]
pub(crate) const FLOAT_64_ARRAY_DISCRIMINANT: u8 =
    value_discriminant(Value::Float64Array(TypedArrayIndex::from_u32_index(0)));
pub(crate) const ASYNC_FROM_SYNC_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::AsyncFromSyncIterator);
pub(crate) const ASYNC_GENERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::AsyncGenerator(AsyncGenerator::_def()));
pub(crate) const ARRAY_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::ArrayIterator(ArrayIterator::_def()));
#[cfg(feature = "set")]
pub(crate) const SET_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::SetIterator(SetIterator::_def()));
pub(crate) const MAP_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::MapIterator(MapIterator::_def()));
pub(crate) const STRING_ITERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::StringIterator(StringIterator::_def()));
pub(crate) const GENERATOR_DISCRIMINANT: u8 =
    value_discriminant(Value::Generator(Generator::_def()));
pub(crate) const MODULE_DISCRIMINANT: u8 = value_discriminant(Value::Module(Module::_def()));
pub(crate) const EMBEDDER_OBJECT_DISCRIMINANT: u8 =
    value_discriminant(Value::EmbedderObject(EmbedderObject::_def()));

impl<'a> Value<'a> {
    /// Scope a stack-only Value. Stack-only Values are primitives that do not
    /// need to store any data on the heap, hence scoping them is effectively a
    /// no-op. These Values are also not concerned with the garbage collector.
    ///
    /// ## Panics
    ///
    /// If the Value is not stack-only, this method will panic.
    pub const fn scope_static<'scope>(
        self,
        _gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Value<'static>> {
        let key_root_repr = match self {
            Value::Undefined => ValueRootRepr::Undefined,
            Value::Null => ValueRootRepr::Null,
            Value::Boolean(bool) => ValueRootRepr::Boolean(bool),
            Value::SmallString(small_string) => ValueRootRepr::SmallString(small_string),
            Value::Integer(small_integer) => ValueRootRepr::Integer(small_integer),
            Value::SmallF64(small_string) => ValueRootRepr::SmallF64(small_string),
            Value::SmallBigInt(small_string) => ValueRootRepr::SmallBigInt(small_string),
            _ => panic!("Value required rooting"),
        };
        Scoped::from_root_repr(key_root_repr)
    }

    pub fn from_str(agent: &mut Agent, str: &str, gc: NoGcScope<'a, '_>) -> Value<'a> {
        String::from_str(agent, str, gc).into_value()
    }

    pub fn from_string(
        agent: &mut Agent,
        string: std::string::String,
        gc: NoGcScope<'a, '_>,
    ) -> Value<'a> {
        String::from_string(agent, string, gc).into_value()
    }

    pub fn from_static_str(
        agent: &mut Agent,
        str: &'static str,
        gc: NoGcScope<'a, '_>,
    ) -> Value<'a> {
        String::from_static_str(agent, str, gc).into_value()
    }

    pub fn from_f64(agent: &mut Agent, value: f64, gc: NoGcScope<'a, '_>) -> Value<'a> {
        Number::from_f64(agent, value, gc).into_value()
    }

    pub fn from_i64(agent: &mut Agent, value: i64, gc: NoGcScope<'a, '_>) -> Value<'a> {
        Number::from_i64(agent, value, gc).into_value()
    }

    pub fn nan() -> Self {
        Number::nan().into_value()
    }

    pub fn pos_inf() -> Self {
        Number::pos_inf().into_value()
    }

    pub fn neg_inf() -> Self {
        Number::neg_inf().into_value()
    }

    pub fn pos_zero() -> Self {
        Number::pos_zero().into_value()
    }

    pub fn neg_zero() -> Self {
        Number::neg_zero().into_value()
    }

    pub fn is_true(self) -> bool {
        matches!(self, Value::Boolean(true))
    }

    pub fn is_false(self) -> bool {
        matches!(self, Value::Boolean(false))
    }

    pub fn is_object(self) -> bool {
        super::Object::try_from(self).is_ok()
    }

    pub fn is_function(self) -> bool {
        matches!(
            self,
            Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_)
        )
    }

    pub fn is_primitive(self) -> bool {
        Primitive::try_from(self).is_ok()
    }

    pub fn is_string(self) -> bool {
        matches!(self, Value::String(_) | Value::SmallString(_))
    }

    pub fn is_boolean(self) -> bool {
        matches!(self, Value::Boolean(_))
    }

    pub fn is_null(self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_undefined(self) -> bool {
        matches!(self, Value::Undefined)
    }

    pub fn is_pos_zero(self, agent: &Agent) -> bool {
        Number::try_from(self).is_ok_and(|n| n.is_pos_zero(agent))
            || BigInt::try_from(self).is_ok_and(|n| n.is_zero(agent))
    }

    pub fn is_neg_zero(self, agent: &Agent) -> bool {
        Number::try_from(self).is_ok_and(|n| n.is_neg_zero(agent))
    }

    pub fn is_pos_infinity(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_pos_infinity(agent))
            .unwrap_or(false)
    }

    pub fn is_neg_infinity(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_neg_infinity(agent))
            .unwrap_or(false)
    }

    pub fn is_nan(self, agent: &Agent) -> bool {
        Number::try_from(self)
            .map(|n| n.is_nan(agent))
            .unwrap_or(false)
    }

    pub fn is_bigint(self) -> bool {
        matches!(self, Value::BigInt(_) | Value::SmallBigInt(_))
    }

    pub fn is_symbol(self) -> bool {
        matches!(self, Value::Symbol(_))
    }

    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            Value::Number(_)
                | Value::SmallF64(_)
                | Value::Integer(_)
                | Value::BigInt(_)
                | Value::SmallBigInt(_)
        )
    }

    pub fn is_number(self) -> bool {
        matches!(
            self,
            Value::Number(_) | Value::SmallF64(_) | Value::Integer(_)
        )
    }

    pub fn is_integer(self) -> bool {
        matches!(self, Value::Integer(_))
    }

    pub fn is_empty_string(self) -> bool {
        if let Value::SmallString(s) = self {
            s.is_empty()
        } else {
            false
        }
    }

    pub fn to_number<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Number<'gc>> {
        to_number(agent, self, gc)
    }

    pub fn to_bigint<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, BigInt<'gc>> {
        to_big_int(agent, self, gc)
    }

    pub fn to_numeric<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Numeric<'gc>> {
        to_numeric(agent, self, gc)
    }

    pub fn to_int32<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, i32> {
        to_int32(agent, self, gc)
    }

    pub fn to_uint32<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, u32> {
        to_uint32(agent, self, gc)
    }

    pub fn to_int16<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, i16> {
        to_int16(agent, self, gc)
    }

    pub fn to_uint16<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, u16> {
        to_uint16(agent, self, gc)
    }

    pub fn to_string<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, String<'gc>> {
        to_string(agent, self, gc)
    }

    pub fn try_to_string<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<JsResult<'gc, String<'gc>>> {
        try_to_string(agent, self, gc)
    }

    /// A string conversion that will never throw, meant for things like
    /// displaying exceptions.
    pub fn string_repr<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> String<'gc> {
        if let Value::Symbol(symbol_idx) = self {
            // ToString of a symbol always throws. We use the descriptive
            // string instead (the result of `String(symbol)`).
            let gc = gc.into_nogc();
            return symbol_idx.unbind().descriptive_string(agent, gc);
        };
        match self.to_string(agent, gc) {
            Ok(result) => result,
            _ => map_object_to_static_string_repr(self),
        }
    }

    /// A string conversion that will never throw, meant for things like
    /// displaying exceptions.
    pub fn try_string_repr<'gc>(self, agent: &mut Agent, gc: NoGcScope<'gc, '_>) -> String<'gc> {
        if let Value::Symbol(symbol_idx) = self {
            // ToString of a symbol always throws. We use the descriptive
            // string instead (the result of `String(symbol)`).
            return symbol_idx.unbind().descriptive_string(agent, gc);
        };
        match self.try_to_string(agent, gc) {
            TryResult::Continue(result) => result.unwrap(),
            _ => map_object_to_static_string_repr(self),
        }
    }

    /// ### [ℝ](https://tc39.es/ecma262/#%E2%84%9D)
    pub fn to_real<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> JsResult<'gc, f64> {
        Ok(match self {
            Value::Number(n) => agent[n],
            Value::Integer(i) => i.into_i64() as f64,
            Value::SmallF64(f) => f.into_f64(),
            // NOTE: Converting to a number should give us a nice error message.
            _ => to_number(agent, self, gc)?.into_f64(agent),
        })
    }

    pub(crate) fn hash<H, A>(self, arena: &A, hasher: &mut H)
    where
        H: Hasher,
        A: Index<HeapString<'a>, Output = StringHeapData>
            + Index<HeapNumber<'a>, Output = f64>
            + Index<HeapBigInt<'a>, Output = BigIntHeapData>,
    {
        let discriminant = core::mem::discriminant(&self);
        match self {
            Value::Undefined => discriminant.hash(hasher),
            Value::Null => discriminant.hash(hasher),
            Value::Boolean(data) => {
                discriminant.hash(hasher);
                data.hash(hasher);
            }
            Value::String(data) => {
                // Skip discriminant hashing in strings
                arena[data].data.hash(hasher);
            }
            Value::SmallString(data) => {
                data.as_wtf8().hash(hasher);
            }
            Value::Symbol(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Number(data) => {
                // Skip discriminant hashing in numbers
                arena[data].to_bits().hash(hasher);
            }
            Value::Integer(data) => {
                data.into_i64().hash(hasher);
            }
            Value::SmallF64(data) => {
                data.into_f64().to_bits().hash(hasher);
            }
            Value::BigInt(data) => {
                // Skip dsciriminant hashing in bigint numbers
                arena[data].data.hash(hasher);
            }
            Value::SmallBigInt(data) => {
                data.into_i64().hash(hasher);
            }
            Value::Object(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BoundFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::ECMAScriptFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinPromiseResolvingFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::PrimitiveObject(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Arguments(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Array(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "date")]
            Value::Date(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Error(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::FinalizationRegistry(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Map(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Promise(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Proxy(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "regexp")]
            Value::RegExp(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "set")]
            Value::Set(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncGenerator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::ArrayIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "set")]
            Value::SetIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::MapIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::StringIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Generator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Module(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::EmbedderObject(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
        };
    }

    pub(crate) fn try_hash<H>(self, hasher: &mut H) -> Result<(), ()>
    where
        H: Hasher,
    {
        let discriminant = core::mem::discriminant(&self);
        match self {
            Value::String(_) | Value::Number(_) | Value::BigInt(_) => {
                // These values need Agent access to hash.
                return Err(());
            }
            // All other types can be hashed on the stack.
            Value::Undefined => discriminant.hash(hasher),
            Value::Null => discriminant.hash(hasher),
            Value::Boolean(data) => {
                discriminant.hash(hasher);
                data.hash(hasher);
            }
            Value::SmallString(data) => {
                data.to_string_lossy().hash(hasher);
            }
            Value::Symbol(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Integer(data) => {
                data.into_i64().hash(hasher);
            }
            Value::SmallF64(data) => {
                data.into_f64().to_bits().hash(hasher);
            }
            Value::SmallBigInt(data) => {
                data.into_i64().hash(hasher);
            }
            Value::Object(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BoundFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::ECMAScriptFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinPromiseResolvingFunction(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::PrimitiveObject(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Arguments(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Array(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "date")]
            Value::Date(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Error(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::FinalizationRegistry(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Map(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Promise(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Proxy(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "regexp")]
            Value::RegExp(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "set")]
            Value::Set(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => {
                discriminant.hash(hasher);
                data.into_index().hash(hasher);
            }
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncGenerator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::ArrayIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            #[cfg(feature = "set")]
            Value::SetIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::MapIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::StringIterator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Generator(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::Module(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
            Value::EmbedderObject(data) => {
                discriminant.hash(hasher);
                data.get_index().hash(hasher);
            }
        }
        Ok(())
    }
}

impl From<bool> for Value<'_> {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Value<'_> {
    type Of<'a> = Value<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a, T> From<Option<T>> for Value<'a>
where
    T: Into<Value<'a>>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(Value::Undefined, |v| v.into())
    }
}

impl TryFrom<&str> for Value<'static> {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, ()> {
        if let Ok(data) = value.try_into() {
            Ok(Value::SmallString(data))
        } else {
            Err(())
        }
    }
}

impl TryFrom<f64> for Value<'static> {
    type Error = ();
    fn try_from(value: f64) -> Result<Self, ()> {
        Number::try_from(value).map(|v| v.into())
    }
}

impl<'a> From<Number<'a>> for Value<'a> {
    fn from(value: Number<'a>) -> Self {
        match value {
            Number::Number(idx) => Value::Number(idx.unbind()),
            Number::Integer(data) => Value::Integer(data),
            Number::SmallF64(data) => Value::SmallF64(data),
        }
    }
}

impl From<f32> for Value<'static> {
    fn from(value: f32) -> Self {
        Value::SmallF64(SmallF64::from(value))
    }
}

impl TryFrom<i64> for Value<'static> {
    type Error = ();
    fn try_from(value: i64) -> Result<Self, ()> {
        Ok(Value::Integer(SmallInteger::try_from(value)?))
    }
}

impl<'a> TryFrom<Value<'a>> for bool {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Boolean(bool) => Ok(bool),
            _ => Err(()),
        }
    }
}

macro_rules! impl_value_from_n {
    ($size: ty) => {
        impl From<$size> for Value<'_> {
            fn from(value: $size) -> Self {
                Value::Integer(SmallInteger::from(value))
            }
        }
    };
}

impl_value_from_n!(u8);
impl_value_from_n!(i8);
impl_value_from_n!(u16);
impl_value_from_n!(i16);
impl_value_from_n!(u32);
impl_value_from_n!(i32);

impl Rootable for Value<'_> {
    type RootRepr = ValueRootRepr;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Undefined => Ok(Self::RootRepr::Undefined),
            Self::Null => Ok(Self::RootRepr::Null),
            Self::Boolean(bool) => Ok(Self::RootRepr::Boolean(bool)),
            Self::String(heap_string) => Err(HeapRootData::String(heap_string.unbind())),
            Self::SmallString(small_string) => Ok(Self::RootRepr::SmallString(small_string)),
            Self::Symbol(symbol) => Err(HeapRootData::Symbol(symbol.unbind())),
            Self::Number(heap_number) => Err(HeapRootData::Number(heap_number.unbind())),
            Self::Integer(small_integer) => Ok(Self::RootRepr::Integer(small_integer)),
            Self::SmallF64(small_f64) => Ok(Self::RootRepr::SmallF64(small_f64)),
            Self::BigInt(heap_big_int) => Err(HeapRootData::BigInt(heap_big_int.unbind())),
            Self::SmallBigInt(small_big_int) => Ok(Self::RootRepr::SmallBigInt(small_big_int)),
            Self::Object(ordinary_object) => Err(HeapRootData::Object(ordinary_object.unbind())),
            Self::BoundFunction(bound_function) => {
                Err(HeapRootData::BoundFunction(bound_function.unbind()))
            }
            Self::BuiltinFunction(builtin_function) => {
                Err(HeapRootData::BuiltinFunction(builtin_function.unbind()))
            }
            Self::ECMAScriptFunction(ecmascript_function) => Err(HeapRootData::ECMAScriptFunction(
                ecmascript_function.unbind(),
            )),
            Self::BuiltinGeneratorFunction => Err(HeapRootData::BuiltinGeneratorFunction),
            Self::BuiltinConstructorFunction(builtin_constructor_function) => Err(
                HeapRootData::BuiltinConstructorFunction(builtin_constructor_function.unbind()),
            ),
            Self::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Err(HeapRootData::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function.unbind(),
                ))
            }
            Self::BuiltinPromiseCollectorFunction => {
                Err(HeapRootData::BuiltinPromiseCollectorFunction)
            }
            Self::BuiltinProxyRevokerFunction => Err(HeapRootData::BuiltinProxyRevokerFunction),
            Self::PrimitiveObject(primitive_object) => {
                Err(HeapRootData::PrimitiveObject(primitive_object.unbind()))
            }
            Self::Arguments(ordinary_object) => {
                Err(HeapRootData::Arguments(ordinary_object.unbind()))
            }
            Self::Array(array) => Err(HeapRootData::Array(array.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::ArrayBuffer(array_buffer) => {
                Err(HeapRootData::ArrayBuffer(array_buffer.unbind()))
            }
            #[cfg(feature = "array-buffer")]
            Self::DataView(data_view) => Err(HeapRootData::DataView(data_view.unbind())),
            #[cfg(feature = "date")]
            Self::Date(date) => Err(HeapRootData::Date(date.unbind())),
            Self::Error(error) => Err(HeapRootData::Error(error.unbind())),
            Self::FinalizationRegistry(finalization_registry) => Err(
                HeapRootData::FinalizationRegistry(finalization_registry.unbind()),
            ),
            Self::Map(map) => Err(HeapRootData::Map(map.unbind())),
            Self::Promise(promise) => Err(HeapRootData::Promise(promise.unbind())),
            Self::Proxy(proxy) => Err(HeapRootData::Proxy(proxy.unbind())),
            #[cfg(feature = "regexp")]
            Self::RegExp(reg_exp) => Err(HeapRootData::RegExp(reg_exp.unbind())),
            #[cfg(feature = "set")]
            Self::Set(set) => Err(HeapRootData::Set(set.unbind())),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(shared_array_buffer) => Err(HeapRootData::SharedArrayBuffer(
                shared_array_buffer.unbind(),
            )),
            #[cfg(feature = "weak-refs")]
            Self::WeakMap(weak_map) => Err(HeapRootData::WeakMap(weak_map.unbind())),
            #[cfg(feature = "weak-refs")]
            Self::WeakRef(weak_ref) => Err(HeapRootData::WeakRef(weak_ref.unbind())),
            #[cfg(feature = "weak-refs")]
            Self::WeakSet(weak_set) => Err(HeapRootData::WeakSet(weak_set.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Int8Array(base_index) => Err(HeapRootData::Int8Array(base_index.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Uint8Array(base_index) => Err(HeapRootData::Uint8Array(base_index.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Uint8ClampedArray(base_index) => {
                Err(HeapRootData::Uint8ClampedArray(base_index.unbind()))
            }
            #[cfg(feature = "array-buffer")]
            Self::Int16Array(base_index) => Err(HeapRootData::Int16Array(base_index.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Uint16Array(base_index) => Err(HeapRootData::Uint16Array(base_index.unbind())),
            #[cfg(feature = "proposal-float16array")]
            Self::Float16Array(base_index) => Err(HeapRootData::Float16Array(base_index.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Int32Array(base_index) => Err(HeapRootData::Int32Array(base_index.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Uint32Array(base_index) => Err(HeapRootData::Uint32Array(base_index.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::Float32Array(base_index) => Err(HeapRootData::Float32Array(base_index.unbind())),
            #[cfg(feature = "array-buffer")]
            Self::BigInt64Array(base_index) => {
                Err(HeapRootData::BigInt64Array(base_index.unbind()))
            }
            #[cfg(feature = "array-buffer")]
            Self::BigUint64Array(base_index) => {
                Err(HeapRootData::BigUint64Array(base_index.unbind()))
            }
            #[cfg(feature = "array-buffer")]
            Self::Float64Array(base_index) => Err(HeapRootData::Float64Array(base_index.unbind())),
            Self::AsyncFromSyncIterator => Err(HeapRootData::AsyncFromSyncIterator),
            Self::AsyncGenerator(r#gen) => Err(HeapRootData::AsyncGenerator(r#gen.unbind())),

            Self::ArrayIterator(array_iterator) => {
                Err(HeapRootData::ArrayIterator(array_iterator.unbind()))
            }
            #[cfg(feature = "set")]
            Self::SetIterator(set_iterator) => {
                Err(HeapRootData::SetIterator(set_iterator.unbind()))
            }
            Self::MapIterator(map_iterator) => {
                Err(HeapRootData::MapIterator(map_iterator.unbind()))
            }
            Self::Generator(generator) => Err(HeapRootData::Generator(generator.unbind())),
            Self::StringIterator(generator) => {
                Err(HeapRootData::StringIterator(generator.unbind()))
            }
            Self::Module(module) => Err(HeapRootData::Module(module.unbind())),
            Self::EmbedderObject(embedder_object) => {
                Err(HeapRootData::EmbedderObject(embedder_object.unbind()))
            }
        }
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match *value {
            Self::RootRepr::Undefined => Ok(Self::Undefined),
            Self::RootRepr::Null => Ok(Self::Null),
            Self::RootRepr::Boolean(bool) => Ok(Self::Boolean(bool)),
            Self::RootRepr::SmallString(small_string) => Ok(Self::SmallString(small_string)),
            Self::RootRepr::Integer(small_integer) => Ok(Self::Integer(small_integer)),
            Self::RootRepr::SmallF64(small_f64) => Ok(Self::SmallF64(small_f64)),
            Self::RootRepr::SmallBigInt(small_big_int) => Ok(Self::SmallBigInt(small_big_int)),
            Self::RootRepr::HeapRef(heap_root_ref) => Err(heap_root_ref),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        Self::RootRepr::HeapRef(heap_ref)
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Empty => None,
            HeapRootData::String(heap_string) => Some(Self::String(heap_string)),
            HeapRootData::Symbol(symbol) => Some(Self::Symbol(symbol)),
            HeapRootData::Number(heap_number) => Some(Self::Number(heap_number)),
            HeapRootData::BigInt(heap_big_int) => Some(Self::BigInt(heap_big_int)),
            HeapRootData::Object(ordinary_object) => Some(Self::Object(ordinary_object)),
            HeapRootData::BoundFunction(bound_function) => {
                Some(Self::BoundFunction(bound_function))
            }
            HeapRootData::BuiltinFunction(builtin_function) => {
                Some(Self::BuiltinFunction(builtin_function))
            }
            HeapRootData::ECMAScriptFunction(ecmascript_function) => {
                Some(Self::ECMAScriptFunction(ecmascript_function))
            }
            HeapRootData::BuiltinGeneratorFunction => Some(Self::BuiltinGeneratorFunction),
            HeapRootData::BuiltinConstructorFunction(builtin_constructor_function) => Some(
                Self::BuiltinConstructorFunction(builtin_constructor_function),
            ),
            HeapRootData::BuiltinPromiseResolvingFunction(builtin_promise_resolving_function) => {
                Some(Self::BuiltinPromiseResolvingFunction(
                    builtin_promise_resolving_function,
                ))
            }
            HeapRootData::BuiltinPromiseCollectorFunction => {
                Some(Self::BuiltinPromiseCollectorFunction)
            }
            HeapRootData::BuiltinProxyRevokerFunction => Some(Self::BuiltinProxyRevokerFunction),
            HeapRootData::PrimitiveObject(primitive_object) => {
                Some(Self::PrimitiveObject(primitive_object))
            }
            HeapRootData::Arguments(ordinary_object) => Some(Self::Arguments(ordinary_object)),
            HeapRootData::Array(array) => Some(Self::Array(array)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::ArrayBuffer(array_buffer) => Some(Self::ArrayBuffer(array_buffer)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::DataView(data_view) => Some(Self::DataView(data_view)),
            #[cfg(feature = "date")]
            HeapRootData::Date(date) => Some(Self::Date(date)),
            HeapRootData::Error(error) => Some(Self::Error(error)),
            HeapRootData::FinalizationRegistry(finalization_registry) => {
                Some(Self::FinalizationRegistry(finalization_registry))
            }
            HeapRootData::Map(map) => Some(Self::Map(map)),
            HeapRootData::Promise(promise) => Some(Self::Promise(promise)),
            HeapRootData::Proxy(proxy) => Some(Self::Proxy(proxy)),
            #[cfg(feature = "regexp")]
            HeapRootData::RegExp(reg_exp) => Some(Self::RegExp(reg_exp)),
            #[cfg(feature = "set")]
            HeapRootData::Set(set) => Some(Self::Set(set)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(shared_array_buffer) => {
                Some(Self::SharedArrayBuffer(shared_array_buffer))
            }
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakMap(weak_map) => Some(Self::WeakMap(weak_map)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakRef(weak_ref) => Some(Self::WeakRef(weak_ref)),
            #[cfg(feature = "weak-refs")]
            HeapRootData::WeakSet(weak_set) => Some(Self::WeakSet(weak_set)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int8Array(base_index) => Some(Self::Int8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8Array(base_index) => Some(Self::Uint8Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint8ClampedArray(base_index) => {
                Some(Self::Uint8ClampedArray(base_index))
            }
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int16Array(base_index) => Some(Self::Int16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint16Array(base_index) => Some(Self::Uint16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Int32Array(base_index) => Some(Self::Int32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Uint32Array(base_index) => Some(Self::Uint32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigInt64Array(base_index) => Some(Self::BigInt64Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::BigUint64Array(base_index) => Some(Self::BigUint64Array(base_index)),
            #[cfg(feature = "proposal-float16array")]
            HeapRootData::Float16Array(base_index) => Some(Self::Float16Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float32Array(base_index) => Some(Self::Float32Array(base_index)),
            #[cfg(feature = "array-buffer")]
            HeapRootData::Float64Array(base_index) => Some(Self::Float64Array(base_index)),
            HeapRootData::AsyncFromSyncIterator => Some(Self::AsyncFromSyncIterator),
            HeapRootData::AsyncGenerator(r#gen) => Some(Self::AsyncGenerator(r#gen)),

            HeapRootData::ArrayIterator(array_iterator) => {
                Some(Self::ArrayIterator(array_iterator))
            }
            #[cfg(feature = "set")]
            HeapRootData::SetIterator(set_iterator) => Some(Self::SetIterator(set_iterator)),
            HeapRootData::MapIterator(map_iterator) => Some(Self::MapIterator(map_iterator)),
            HeapRootData::StringIterator(generator) => Some(Self::StringIterator(generator)),
            HeapRootData::Generator(generator) => Some(Self::Generator(generator)),
            HeapRootData::Module(module) => Some(Self::Module(module)),
            HeapRootData::EmbedderObject(embedder_object) => {
                Some(Self::EmbedderObject(embedder_object))
            }
            HeapRootData::Executable(_)
            | HeapRootData::Realm(_)
            | HeapRootData::Script(_)
            | HeapRootData::SourceCode(_)
            | HeapRootData::SourceTextModule(_)
            | HeapRootData::AwaitReaction(_)
            | HeapRootData::PromiseReaction(_)
            | HeapRootData::DeclarativeEnvironment(_)
            | HeapRootData::FunctionEnvironment(_)
            | HeapRootData::GlobalEnvironment(_)
            | HeapRootData::ModuleEnvironment(_)
            | HeapRootData::ObjectEnvironment(_)
            | HeapRootData::PrivateEnvironment(_) => None,
            // Note: Do not use _ => Err(()) to make sure any added
            // HeapRootData Value variants cause compile errors if not handled.
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ValueRootRepr {
    Undefined = UNDEFINED_DISCRIMINANT,
    Null = NULL_DISCRIMINANT,
    Boolean(bool) = BOOLEAN_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallF64(SmallF64) = FLOAT_DISCRIMINANT,
    SmallBigInt(SmallBigInt) = SMALL_BIGINT_DISCRIMINANT,
    HeapRef(HeapRootRef) = 0x80,
}

impl HeapMarkAndSweep for Value<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::SmallString(_)
            | Value::Integer(_)
            | Value::SmallF64(_)
            | Value::SmallBigInt(_) => {
                // Stack values: Nothing to mark
            }
            Value::String(data) => data.mark_values(queues),
            Value::Symbol(data) => data.mark_values(queues),
            Value::Number(data) => data.mark_values(queues),
            Value::BigInt(data) => data.mark_values(queues),
            Value::Object(data) => data.mark_values(queues),
            Value::Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "date")]
            Value::Date(data) => data.mark_values(queues),
            Value::Error(data) => data.mark_values(queues),
            Value::BoundFunction(data) => data.mark_values(queues),
            Value::BuiltinFunction(data) => data.mark_values(queues),
            Value::ECMAScriptFunction(data) => data.mark_values(queues),
            #[cfg(feature = "regexp")]
            Value::RegExp(data) => data.mark_values(queues),
            Value::PrimitiveObject(data) => data.mark_values(queues),
            Value::Arguments(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => data.mark_values(queues),
            Value::FinalizationRegistry(data) => data.mark_values(queues),
            Value::Map(data) => data.mark_values(queues),
            Value::Proxy(data) => data.mark_values(queues),
            Value::Promise(data) => data.mark_values(queues),
            #[cfg(feature = "set")]
            Value::Set(data) => data.mark_values(queues),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => data.mark_values(queues),
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => data.mark_values(queues),
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => data.mark_values(queues),
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => data.mark_values(queues),
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction(data) => data.mark_values(queues),
            Value::BuiltinPromiseResolvingFunction(data) => data.mark_values(queues),
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncGenerator(data) => data.mark_values(queues),
            Value::ArrayIterator(data) => data.mark_values(queues),
            #[cfg(feature = "set")]
            Value::SetIterator(data) => data.mark_values(queues),
            Value::MapIterator(data) => data.mark_values(queues),
            Value::StringIterator(data) => data.mark_values(queues),
            Value::Generator(data) => data.mark_values(queues),
            Value::Module(data) => data.mark_values(queues),
            Value::EmbedderObject(data) => data.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::SmallString(_)
            | Value::Integer(_)
            | Value::SmallF64(_)
            | Value::SmallBigInt(_) => {
                // Stack values: Nothing to sweep
            }
            Value::String(data) => data.sweep_values(compactions),
            Value::Symbol(data) => data.sweep_values(compactions),
            Value::Number(data) => data.sweep_values(compactions),
            Value::BigInt(data) => data.sweep_values(compactions),
            Value::Object(data) => data.sweep_values(compactions),
            Value::Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::ArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "date")]
            Value::Date(data) => data.sweep_values(compactions),
            Value::Error(data) => data.sweep_values(compactions),
            Value::BoundFunction(data) => data.sweep_values(compactions),
            Value::BuiltinFunction(data) => data.sweep_values(compactions),
            Value::ECMAScriptFunction(data) => data.sweep_values(compactions),
            #[cfg(feature = "regexp")]
            Value::RegExp(data) => data.sweep_values(compactions),
            Value::PrimitiveObject(data) => data.sweep_values(compactions),
            Value::Arguments(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::DataView(data) => data.sweep_values(compactions),
            Value::FinalizationRegistry(data) => data.sweep_values(compactions),
            Value::Map(data) => data.sweep_values(compactions),
            Value::Proxy(data) => data.sweep_values(compactions),
            Value::Promise(data) => data.sweep_values(compactions),
            #[cfg(feature = "set")]
            Value::Set(data) => data.sweep_values(compactions),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Value::WeakMap(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Value::WeakRef(data) => data.sweep_values(compactions),
            #[cfg(feature = "weak-refs")]
            Value::WeakSet(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Int8Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Uint8Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Uint8ClampedArray(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Int16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Uint16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Int32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Uint32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::BigInt64Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::BigUint64Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Float32Array(data) => data.sweep_values(compactions),
            #[cfg(feature = "array-buffer")]
            Value::Float64Array(data) => data.sweep_values(compactions),
            Value::BuiltinGeneratorFunction => todo!(),
            Value::BuiltinConstructorFunction(data) => data.sweep_values(compactions),
            Value::BuiltinPromiseResolvingFunction(data) => data.sweep_values(compactions),
            Value::BuiltinPromiseCollectorFunction => todo!(),
            Value::BuiltinProxyRevokerFunction => todo!(),
            Value::AsyncFromSyncIterator => todo!(),
            Value::AsyncGenerator(data) => data.sweep_values(compactions),
            Value::ArrayIterator(data) => data.sweep_values(compactions),
            #[cfg(feature = "set")]
            Value::SetIterator(data) => data.sweep_values(compactions),
            Value::MapIterator(data) => data.sweep_values(compactions),
            Value::StringIterator(data) => data.sweep_values(compactions),
            Value::Generator(data) => data.sweep_values(compactions),
            Value::Module(data) => data.sweep_values(compactions),
            Value::EmbedderObject(data) => data.sweep_values(compactions),
        }
    }
}

fn map_object_to_static_string_repr(value: Value) -> String<'static> {
    match Object::try_from(value).unwrap() {
        Object::BoundFunction(_)
        | Object::BuiltinFunction(_)
        | Object::ECMAScriptFunction(_)
        | Object::BuiltinGeneratorFunction
        | Object::BuiltinConstructorFunction(_)
        | Object::BuiltinPromiseResolvingFunction(_)
        | Object::BuiltinPromiseCollectorFunction
        | Object::BuiltinProxyRevokerFunction => BUILTIN_STRING_MEMORY._object_Function_,
        Object::Arguments(_) => BUILTIN_STRING_MEMORY._object_Arguments_,
        Object::Array(_) => BUILTIN_STRING_MEMORY._object_Array_,
        Object::Error(_) => BUILTIN_STRING_MEMORY._object_Error_,
        Object::RegExp(_) => BUILTIN_STRING_MEMORY._object_RegExp_,
        Object::Module(_) => BUILTIN_STRING_MEMORY._object_Module_,
        Object::Object(_)
        | Object::PrimitiveObject(_)
        | Object::ArrayBuffer(_)
        | Object::DataView(_)
        | Object::Date(_)
        | Object::FinalizationRegistry(_)
        | Object::Map(_)
        | Object::Promise(_)
        | Object::Proxy(_)
        | Object::Set(_)
        | Object::SharedArrayBuffer(_)
        | Object::WeakMap(_)
        | Object::WeakRef(_)
        | Object::WeakSet(_)
        | Object::Int8Array(_)
        | Object::Uint8Array(_)
        | Object::Uint8ClampedArray(_)
        | Object::Int16Array(_)
        | Object::Uint16Array(_)
        | Object::Int32Array(_)
        | Object::Uint32Array(_)
        | Object::BigInt64Array(_)
        | Object::BigUint64Array(_)
        | Object::Float32Array(_)
        | Object::Float64Array(_)
        | Object::AsyncFromSyncIterator
        | Object::AsyncGenerator(_)
        | Object::ArrayIterator(_)
        | Object::SetIterator(_)
        | Object::MapIterator(_)
        | Object::StringIterator(_)
        | Object::Generator(_)
        | Object::EmbedderObject(_) => BUILTIN_STRING_MEMORY._object_Object_,
        #[cfg(feature = "proposal-float16array")]
        Object::Float16Array(_) => BUILTIN_STRING_MEMORY._object_Object_,
    }
}

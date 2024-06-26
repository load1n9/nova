use crate::{
    ecmascript::{
        abstract_operations::{
            testing_and_comparison::same_value_zero,
            type_conversion::{to_number, to_uint32},
        },
        builtins::ordinary::ordinary_define_own_property,
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{
            InternalMethods, IntoObject, Object, PropertyDescriptor, PropertyKey,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::indexes::ArrayIndex,
};

use super::{data::SealableElementsVector, Array, ArrayHeapData};

/// ### [10.4.2.2 ArrayCreate ( length \[ , proto \] )](https://tc39.es/ecma262/#sec-arraycreate)
///
/// The abstract operation ArrayCreate takes argument length (a non-negative
/// integer) and optional argument proto (an Object) and returns either a
/// normal completion containing an Array exotic object or a throw completion.
/// It is used to specify the creation of new Arrays.
pub fn array_create(
    agent: &mut Agent,
    length: usize,
    capacity: usize,
    proto: Option<Object>,
) -> JsResult<Array> {
    // 1. If length > 2**32 - 1, throw a RangeError exception.
    if length > (2usize.pow(32) - 1) {
        return Err(agent.throw_exception(ExceptionType::RangeError, "invalid array length"));
    }
    // 2. If proto is not present, set proto to %Array.prototype%.
    let object_index = if let Some(proto) = proto {
        if proto
            == agent
                .current_realm()
                .intrinsics()
                .array_prototype()
                .into_object()
        {
            None
        } else {
            Some(agent.heap.create_object_with_prototype(proto, &[]))
        }
    } else {
        None
    };
    // 3. Let A be MakeBasicObject(« [[Prototype]], [[Extensible]] »).
    // 5. Set A.[[DefineOwnProperty]] as specified in 10.4.2.1.
    let elements = agent
        .heap
        .elements
        .allocate_elements_with_capacity(capacity);
    let data = ArrayHeapData {
        // 4. Set A.[[Prototype]] to proto.
        object_index,
        elements: SealableElementsVector::from_elements_vector(elements),
    };
    agent.heap.arrays.push(Some(data));

    // 7. Return A.
    Ok(Array(ArrayIndex::last(&agent.heap.arrays)))
}

/// ### [10.4.2.4 ArraySetLength ( A, Desc )](https://tc39.es/ecma262/#sec-arraysetlength)
///
/// The abstract operation ArraySetLength takes arguments A (an Array) and Desc (a Property Descriptor) and returns either a normal completion containing a Boolean or a throw completion.
pub fn array_set_length(agent: &mut Agent, a: Array, desc: PropertyDescriptor) -> JsResult<bool> {
    // 1. If Desc does not have a [[Value]] field, then
    let length_key = PropertyKey::from(BUILTIN_STRING_MEMORY.length);
    if desc.value.is_none() {
        // a. Return ! OrdinaryDefineOwnProperty(A, "length", Desc).
        return Ok(ordinary_define_own_property(agent, a.into(), length_key, desc).unwrap());
    }
    let desc_value = desc.value.unwrap();
    // 2. Let newLenDesc be a copy of Desc.
    // 13. If newLenDesc does not have a [[Writable]] field or newLenDesc.[[Writable]] is true, then
    // a. Let newLenDesc.[[Writable]] be true
    let new_len_writable = desc.writable.unwrap_or(true);
    // NOTE: Setting the [[Writable]] attribute to false is deferred in case any elements cannot be deleted.
    // 3. Let newLen be ? ToUint32(Desc.[[Value]]).
    let new_len = to_uint32(agent, desc_value)?;
    // 4. Let numberLen be ? ToNumber(Desc.[[Value]]).
    let number_len = to_number(agent, desc_value)?;
    // 5. If SameValueZero(newLen, numberLen) is false, throw a RangeError exception.
    if same_value_zero(agent, new_len, number_len) {
        return Err(agent.throw_exception(ExceptionType::RangeError, "invalid array length"));
    }
    // 6. Set newLenDesc.[[Value]] to newLen.
    // 7. Let oldLenDesc be OrdinaryGetOwnProperty(A, "length").
    let array_heap_data = &mut agent[a];
    // 10. Let oldLen be oldLenDesc.[[Value]].
    let (old_len, old_len_writable) = (
        array_heap_data.elements.len(),
        array_heap_data.elements.len_writable,
    );
    // 12. If oldLenDesc.[[Writable]] is false, return false.
    if !old_len_writable {
        return Ok(false);
    }
    // Optimization: check OrdinaryDefineOwnProperty conditions for failing early on.
    if desc.configurable == Some(true) || desc.enumerable == Some(true) {
        // 16. If succeeded is false, return false.
        return Ok(false);
    }
    // 11. If newLen ≥ oldLen, then
    if new_len >= old_len {
        // a. Return ! OrdinaryDefineOwnProperty(A, "length", newLenDesc).
        // TODO: Handle growing elements
        array_heap_data.elements.len = new_len;
        array_heap_data.elements.len_writable = new_len_writable;
        return Ok(true);
    }
    // 15. Let succeeded be ! OrdinaryDefineOwnProperty(A, "length", newLenDesc).
    array_heap_data.elements.len = new_len;
    // 17. For each own property key P of A such that P is an array index and ! ToUint32(P) ≥ newLen, in descending numeric index order, do
    debug_assert!(old_len > new_len);
    for i in old_len..new_len {
        // a. Let deleteSucceeded be ! A.[[Delete]](P).
        let delete_succeeded = a
            .internal_delete(agent, PropertyKey::Integer(i.into()))
            .unwrap();
        // b. If deleteSucceeded is false, then
        if !delete_succeeded {
            let array_heap_data = &mut agent[a];
            // i. Set newLenDesc.[[Value]] to ! ToUint32(P) + 1𝔽.
            array_heap_data.elements.len = i + 1;
            // ii. If newWritable is false, set newLenDesc.[[Writable]] to false.
            array_heap_data.elements.len_writable &= new_len_writable;
            // iii. Perform ! OrdinaryDefineOwnProperty(A, "length", newLenDesc).
            // iv. Return false.
            return Ok(false);
        }
    }
    // 18. If newWritable is false, then
    if !new_len_writable {
        // a. Set succeeded to ! OrdinaryDefineOwnProperty(A, "length", PropertyDescriptor { [[Writable]]: false }).
        // b. Assert: succeeded is true.
        let array_heap_data = &mut agent[a];
        array_heap_data.elements.len_writable &= new_len_writable;
    }
    // 19. Return true.
    Ok(true)
}

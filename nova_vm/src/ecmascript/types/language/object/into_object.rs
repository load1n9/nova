use crate::ecmascript::types::language::into_value::IntoValue;

use super::Object;

pub trait IntoObject<'a>
where
    Self: Sized + Copy + IntoValue,
{
    fn into_object(self) -> Object<'a>;
}

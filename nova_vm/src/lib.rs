// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(const_refs_to_cell)]
#![cfg_attr(not(test),no_std)]
#![allow(dead_code)]
extern crate alloc;
pub mod ecmascript;
pub mod engine;
pub mod heap;
pub use engine::small_integer::SmallInteger;
use heap::Heap;
pub use small_string::SmallString;

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::OrdinaryObject,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};
// use std::time::SystemTime;

#[derive(Debug, Clone, Copy)]
pub struct DateHeapData<T> {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) date: Option<T>,
}

impl<T> DateHeapData<T> {
    pub(crate) fn new_invalid() -> Self {
        Self {
            object_index: None,
            date: None,
        }
    }
}

impl<T> HeapMarkAndSweep for DateHeapData<T> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}

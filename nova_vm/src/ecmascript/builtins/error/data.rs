use crate::{
    ecmascript::{
        execution::agent::ExceptionType,
        types::{OrdinaryObject, String, Value},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct ErrorHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) kind: ExceptionType,
    pub(crate) message: Option<String>,
    pub(crate) cause: Option<Value>,
    // TODO: stack? name?
}

impl ErrorHeapData {
    pub(crate) fn new(kind: ExceptionType, message: Option<String>, cause: Option<Value>) -> Self {
        Self {
            object_index: None,
            kind,
            message,
            cause,
        }
    }
}

impl HeapMarkAndSweep for ErrorHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
        self.message.mark_values(queues);
        self.cause.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.message.sweep_values(compactions);
        self.cause.sweep_values(compactions);
    }
}

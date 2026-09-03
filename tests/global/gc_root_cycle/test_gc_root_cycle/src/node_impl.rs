use std::sync::atomic::{AtomicI32, Ordering};

use crate::api::NodeClass;

static FINALIZER_COUNT: AtomicI32 = AtomicI32::new(0);

/// RIDL class `Node`: a JS-visible object whose native box counts its own
/// finalization. Used by GC tests as the "was this cycle collected?" signal.
///
/// NOTE: the reference cycle under test is built on the JS side with plain
/// properties (e.g. `node.held = obj; obj.back = node;`). The v1 RIDL glue
/// does not pass `object` params or the receiver into trait methods, so the
/// native holding methods below are intentionally kept as API surface and
/// behave as no-ops.
pub struct DefaultNode;

impl DefaultNode {
    pub fn new() -> Self {
        Self
    }
}

impl Drop for DefaultNode {
    fn drop(&mut self) {
        FINALIZER_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

impl NodeClass for DefaultNode {
    fn clear_held(&mut self) {}

    fn drop_all(&mut self) {}

    fn finalizer_count(&mut self) -> i32 {
        FINALIZER_COUNT.load(Ordering::SeqCst)
    }

    fn root_held(&mut self) {}

    fn unroot_held(&mut self) {}
}

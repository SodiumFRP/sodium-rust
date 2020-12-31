use crate::impl_::listener::Listener as ListenerImpl;

/// A handle for a listener registered on some [`Cell`][crate::Cell]
/// or [`Stream`][crate::Stream].
pub struct Listener {
    pub impl_: ListenerImpl,
}

impl Listener {
    /// Deregister the listener that was registered so it will no
    /// longer be called back, allowing associated resources to be
    /// garbage-collected.
    pub fn unlisten(&self) {
        self.impl_.unlisten();
    }
}

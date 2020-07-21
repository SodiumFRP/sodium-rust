use crate::impl_::listener::Listener as ListenerImpl;

pub struct Listener {
    pub impl_: ListenerImpl,
}

impl Listener {
    pub fn unlisten(&self) {
        self.impl_.unlisten();
    }
}

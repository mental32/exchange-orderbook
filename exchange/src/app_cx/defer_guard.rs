pub struct DeferGuard<F: FnMut()> {
    f: F,
    active: bool,
}

impl<F: FnMut()> Drop for DeferGuard<F> {
    fn drop(&mut self) {
        if self.active {
            (self.f)();
        }
    }
}

impl<F: FnMut()> DeferGuard<F> {
    pub fn cancel(mut self) {
        self.active = false;
    }
}

#[must_use]
pub fn defer<F: FnMut()>(f: F) -> DeferGuard<F> {
    DeferGuard { f, active: true }
}

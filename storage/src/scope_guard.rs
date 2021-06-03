use core::mem::ManuallyDrop;

pub struct ScopeGuard<T, F: FnOnce(T)> {
    extra: ManuallyDrop<T>,
    func: ManuallyDrop<F>,
}

impl<T, F: FnOnce(T)> Drop for ScopeGuard<T, F> {
    fn drop(&mut self) {
        unsafe {
            let extra = ManuallyDrop::take(&mut self.extra);
            let func = ManuallyDrop::take(&mut self.func);
            func(extra)
        }
    }
}

impl ScopeGuard<(), fn(())> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(func: impl FnOnce()) -> ScopeGuard<(), impl FnOnce(())> { ScopeGuard::with_extra((), move |()| func()) }
}

impl<T, F: FnOnce(T)> ScopeGuard<T, F> {
    pub fn with_extra(extra: T, func: F) -> Self {
        Self {
            extra: ManuallyDrop::new(extra),
            func: ManuallyDrop::new(func),
        }
    }

    pub fn extra_mut(&mut self) -> &mut T { &mut self.extra }

    pub fn defuse(self) {
        unsafe {
            let mut this = ManuallyDrop::new(self);
            let this = &mut *this;
            let func = &mut this.func;
            let _guard = ScopeGuard::new(|| {
                ManuallyDrop::drop(func);
            });
            ManuallyDrop::drop(&mut this.extra);
        }
    }
}

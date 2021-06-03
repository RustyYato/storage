use core::{intrinsics::assume, mem::MaybeUninit};

use crate::{boxed::Box, AllocErr, ResizableStorage, Storage};

pub struct Vec<T, S: Storage = crate::Global> {
    len: usize,
    raw: Box<[MaybeUninit<T>], S>,
}

impl<T> Vec<T> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            raw: Box::empty_slice(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self { Self::with_capacity_in(capacity, crate::Global) }
}

impl<T, S: Storage> Vec<T, S> {
    pub fn new_in(storage: S) -> Self {
        Self {
            len: 0,
            raw: Box::empty_slice_in(storage),
        }
    }

    pub fn with_capacity_in(capacity: usize, storage: S) -> Self {
        Self::try_with_capacity_in(capacity, storage).unwrap_or_else(AllocErr::handle)
    }

    pub fn try_with_capacity_in(capacity: usize, storage: S) -> Result<Self, AllocErr> {
        Ok(Self {
            len: 0,
            raw: Box::try_uninit_slice_in(capacity, storage)?,
        })
    }

    pub fn is_empty(&self) -> bool { self.len == 0 }

    pub fn len(&self) -> usize { self.len }

    pub fn capacity(&self) -> usize { self.raw.len() }

    pub unsafe fn push_unchecked(&mut self, value: T) {
        assume(self.len() < self.capacity());
        self.raw[self.len] = MaybeUninit::new(value);
        self.len += 1;
    }

    pub fn try_push(&mut self, value: T) -> Result<(), T> {
        if self.len() < self.capacity() {
            unsafe { self.push_unchecked(value) }
            Ok(())
        } else {
            Err(value)
        }
    }

    pub unsafe fn pop_unchecked(&mut self) -> T {
        self.len -= 1;
        assume(self.len() < self.capacity());
        self.raw[self.len].as_ptr().read()
    }

    pub fn try_pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { self.pop_unchecked() })
        }
    }
}

impl<T, S: ResizableStorage> Vec<T, S> {
    #[cold]
    #[inline(never)]
    pub fn try_reserve_slow(&mut self, new_capacity: usize) -> Result<(), AllocErr> { self.raw.try_grow(new_capacity) }

    pub fn try_reserve(&mut self, additional: usize) -> Result<&mut [MaybeUninit<T>], AllocErr> {
        let len = self.len();
        if self.capacity().wrapping_sub(len) < additional {
            self.try_reserve_slow(len.wrapping_add(additional))?
        }
        unsafe {
            assume(len == self.len());
            let remaining = self.raw.get_unchecked_mut(len..);
            assume(remaining.len() <= additional);
            Ok(remaining)
        }
    }

    pub fn reserve(&mut self, additional: usize) -> &mut [MaybeUninit<T>] {
        self.try_reserve(additional).unwrap_or_else(AllocErr::handle)
    }

    pub fn push(&mut self, value: T) {
        if self.len().wrapping_add(1) == self.capacity() {
            self.reserve(1);
        }

        unsafe { self.push_unchecked(value) }
    }
}

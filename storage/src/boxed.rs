use crate::{scope_guard::ScopeGuard, AllocErr, ResizableStorage, Storage};
use core::{
    alloc::Layout,
    fmt,
    marker::{PhantomData, Unsize},
    mem::{self, ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr::{self, NonNull, Pointee, Thin},
};

pub struct Box<T: ?Sized + Pointee, S: Storage = crate::Global> {
    handle: S::Handle,
    storage: S,
    meta: T::Metadata,
    __: PhantomData<T>,
}

impl<T: ?Sized + Pointee, S: Storage> Drop for Box<T, S> {
    fn drop(&mut self) {
        unsafe {
            let handle = self.handle;
            let ptr = self.storage.get(handle);
            let ptr = ptr::from_raw_parts::<T>(ptr.as_ptr().cast(), self.meta);
            let layout = Layout::for_value(&*ptr);
            let mut scope =
                ScopeGuard::with_extra(&mut self.storage, move |storage| storage.deallocate(handle, layout));
            let ptr = scope.extra_mut().get_mut(handle);
            let ptr = ptr::from_raw_parts_mut::<T>(ptr.as_ptr().cast(), self.meta);
            ptr.drop_in_place()
        }
    }
}

impl<T: ?Sized + Pointee, S: Storage> Deref for Box<T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let ptr = self.storage.get(self.handle);
            let ptr = ptr::from_raw_parts::<T>(ptr.as_ptr().cast(), self.meta);
            &*ptr
        }
    }
}

impl<T: ?Sized + Pointee, S: Storage> DerefMut for Box<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let ptr = self.storage.get_mut(self.handle);
            let ptr = ptr::from_raw_parts_mut::<T>(ptr.as_ptr().cast(), self.meta);
            &mut *ptr
        }
    }
}

impl<T: Thin> Box<MaybeUninit<T>, crate::SingleStackStorage<T>> {
    pub const UNINIT_STACK: Self = Self {
        __: PhantomData,
        handle: (),
        meta: (),
        storage: crate::SingleStackStorage::new(),
    };
}

impl<T> Box<[T]> {
    pub const fn empty_slice() -> Self {
        Self {
            handle: NonNull::<T>::dangling().cast(),
            storage: crate::Global,
            meta: 0,
            __: PhantomData,
        }
    }
}

impl<T, S: Storage> Box<[T], S> {
    pub fn empty_slice_in(storage: S) -> Self {
        Self {
            handle: unsafe { crate::Handle::dangling(mem::align_of::<T>()) },
            storage,
            meta: 0,
            __: PhantomData,
        }
    }

    /// # Panics
    ///
    /// If layout cannot be computed
    pub fn try_uninit_slice_in(len: usize, mut storage: S) -> Result<Box<[MaybeUninit<T>], S>, AllocErr<S>> {
        let layout = Layout::new::<T>().repeat(len).unwrap().0;
        let memory_block = match storage.allocate(layout) {
            Ok(mb) => mb,
            Err(err) => return Err(err.with(storage)),
        };
        Ok(Box {
            __: PhantomData,
            storage,
            handle: memory_block.handle,
            meta: memory_block.size / mem::size_of::<T>(),
        })
    }
}

impl<T: Thin> Box<T> {
    pub fn new(value: T) -> Self { Self::new_in(value, crate::Global) }
}

impl<T: Thin, S: Storage> Box<T, S> {
    pub fn new_in(value: T, storage: S) -> Self { Self::try_new_in(value, storage).unwrap_or_else(AllocErr::handle) }
    pub fn try_new_in(value: T, storage: S) -> Result<Self, AllocErr> {
        Ok(Self::write(Self::try_uninit_in(storage)?, value))
    }

    pub fn try_uninit_in(mut storage: S) -> Result<Box<MaybeUninit<T>, S>, AllocErr> {
        let memory_block = storage.allocate(Layout::new::<T>())?;
        Ok(Box {
            __: PhantomData,
            storage,
            handle: memory_block.handle,
            meta: (),
        })
    }

    pub fn try_zeroed_in(mut storage: S) -> Result<Box<MaybeUninit<T>, S>, AllocErr> {
        let memory_block = storage.allocate_zeroed(Layout::new::<T>())?;
        Ok(Box {
            __: PhantomData,
            storage,
            handle: memory_block.handle,
            meta: (),
        })
    }

    pub fn write(mut this: Box<MaybeUninit<T>, S>, value: T) -> Self {
        unsafe {
            let ptr = this.storage.get_mut(this.handle);
            ptr.as_ptr().cast::<T>().write(value);
            Self::assume_init(this)
        }
    }

    /// # Safety
    ///
    /// the box must be initialized for `T`
    pub unsafe fn assume_init(this: Box<MaybeUninit<T>, S>) -> Self {
        let this = ManuallyDrop::new(this);
        Self {
            __: PhantomData,
            storage: ptr::read(&this.storage),
            handle: this.handle,
            meta: (),
        }
    }
}

impl<T: ?Sized + Pointee, S: Storage> Box<T, S> {
    pub fn cast<U: ?Sized>(self) -> Box<U, S>
    where
        T: Unsize<U>,
    {
        unsafe {
            let ptr = self.storage.get(self.handle);
            let ptr = ptr::from_raw_parts::<T>(ptr.as_ptr().cast(), self.meta);
            let ptr: *const U = ptr;

            let meta = ptr::metadata(ptr);
            let (handle, _, storage) = Self::into_raw_parts(self);
            Box::from_raw_parts(handle, meta, storage)
        }
    }

    pub fn into_raw_parts(this: Self) -> (S::Handle, T::Metadata, S) {
        unsafe {
            let this = ManuallyDrop::new(this);
            let storage = ptr::read(&this.storage);
            (this.handle, this.meta, storage)
        }
    }

    /// # Safety
    ///
    /// `handle` must refer to a valid allocation from `storage`
    /// with a layout that fits `T` with the associated `meta`
    pub unsafe fn from_raw_parts(handle: S::Handle, meta: T::Metadata, storage: S) -> Self {
        Self {
            handle,
            storage,
            meta,
            __: PhantomData,
        }
    }
}

impl<T, S: ResizableStorage> Box<[MaybeUninit<T>], S> {
    pub fn shrink(&mut self, new_size: usize) { self.try_shrink(new_size).unwrap_or_else(AllocErr::handle) }

    pub fn grow(&mut self, new_size: usize) { self.try_grow(new_size).unwrap_or_else(AllocErr::handle) }

    pub fn grow_zeroed(&mut self, new_size: usize) { self.try_grow_zeroed(new_size).unwrap_or_else(AllocErr::handle) }

    /// # Panics
    ///
    /// if `self.len() < new_size`
    pub fn try_shrink(&mut self, new_size: usize) -> Result<(), AllocErr> {
        unsafe {
            let size = self.len();
            assert!(size >= new_size);
            let old = Layout::from_size_align_unchecked(mem::size_of::<T>() * size, mem::align_of::<T>());
            let new = Layout::from_size_align_unchecked(mem::size_of::<T>() * new_size, mem::align_of::<T>());
            let memory_block = self.storage.shrink(self.handle, old, new)?;
            self.handle = memory_block.handle;
            self.meta = memory_block.size / mem::size_of::<T>();
            Ok(())
        }
    }

    /// # Panics
    ///
    /// if `self.len() > new_size`
    pub fn try_grow(&mut self, new_size: usize) -> Result<(), AllocErr> {
        unsafe {
            let size = self.len();
            assert!(size <= new_size);
            let old = Layout::from_size_align_unchecked(mem::size_of::<T>() * size, mem::align_of::<T>());
            let new = Layout::from_size_align_unchecked(mem::size_of::<T>() * new_size, mem::align_of::<T>());
            let memory_block = self.storage.grow(self.handle, old, new)?;
            self.handle = memory_block.handle;
            self.meta = memory_block.size / mem::size_of::<T>();
            Ok(())
        }
    }

    /// # Panics
    ///
    /// if `self.len() > new_size`
    pub fn try_grow_zeroed(&mut self, new_size: usize) -> Result<(), AllocErr> {
        unsafe {
            let size = self.len();
            assert!(size <= new_size);
            let old = Layout::from_size_align_unchecked(mem::size_of::<T>() * size, mem::align_of::<T>());
            let new = Layout::from_size_align_unchecked(mem::size_of::<T>() * new_size, mem::align_of::<T>());
            let memory_block = self.storage.grow_zeroed(self.handle, old, new)?;
            self.handle = memory_block.handle;
            self.meta = memory_block.size / mem::size_of::<T>();
            Ok(())
        }
    }
}

impl<T: Copy, S: ResizableStorage> Box<[T], S> {
    pub fn shrink_initialized(&mut self, new_size: usize) {
        self.try_shrink_initialized(new_size).unwrap_or_else(AllocErr::handle)
    }

    /// # Panics
    ///
    /// if `self.len() < new_size`
    pub fn try_shrink_initialized(&mut self, new_size: usize) -> Result<(), AllocErr> {
        unsafe {
            let size = self.len();
            assert!(size >= new_size);
            let old = Layout::from_size_align_unchecked(mem::size_of::<T>() * size, mem::align_of::<T>());
            let new = Layout::from_size_align_unchecked(mem::size_of::<T>() * new_size, mem::align_of::<T>());
            let memory_block = self.storage.shrink(self.handle, old, new)?;
            self.handle = memory_block.handle;
            self.meta = memory_block.size / mem::size_of::<T>();
            Ok(())
        }
    }
}

impl<T: fmt::Debug + ?Sized, S: Storage> fmt::Debug for Box<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { T::fmt(self, f) }
}

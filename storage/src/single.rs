use core::{
    alloc::Layout,
    cell::UnsafeCell,
    mem,
    mem::MaybeUninit,
    num::NonZeroUsize,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    AllocErr, FromPtr, MemoryBlock, NonEmptyLayout, NonEmptyMemoryBlock, OffsetHandle, SharedGetMut,
    SharedOffsetHandle, SharedStorage, Storage,
};

pub struct SingleStackStorage<T> {
    memory: UnsafeCell<MaybeUninit<T>>,
    allocated: AtomicBool,
}
pub struct OffsetSingleStackStorage<T> {
    storage: SingleStackStorage<T>,
    offset: UnsafeCell<isize>,
}

unsafe impl<T> Send for SingleStackStorage<T> {}
unsafe impl<T> Sync for SingleStackStorage<T> {}

unsafe impl<T> Send for OffsetSingleStackStorage<T> {}
unsafe impl<T> Sync for OffsetSingleStackStorage<T> {}

impl<T> SingleStackStorage<T> {
    pub const fn new() -> Self {
        Self {
            memory: UnsafeCell::new(MaybeUninit::uninit()),
            allocated: AtomicBool::new(false),
        }
    }

    pub const fn init(value: T) -> Self {
        Self {
            memory: UnsafeCell::new(MaybeUninit::new(value)),
            allocated: AtomicBool::new(false),
        }
    }

    pub const fn offsetable(self) -> OffsetSingleStackStorage<T> {
        OffsetSingleStackStorage {
            offset: UnsafeCell::new(0),
            storage: self,
        }
    }
}

impl<T> SingleStackStorage<T> {
    const fn fits(layout: Layout) -> bool {
        mem::size_of::<T>() >= layout.size() && mem::align_of::<T>() >= layout.align()
    }

    fn aquire(&self) -> bool {
        self.allocated
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }
}

unsafe impl<T> FromPtr for SingleStackStorage<T> {
    unsafe fn from_ptr(&self, _: NonNull<u8>, _: Layout) -> Self::Handle {}
}

unsafe impl<T> SharedGetMut for SingleStackStorage<T> {
    unsafe fn shared_get_mut(&self, _: Self::Handle) -> NonNull<u8> { NonNull::new_unchecked(self.memory.get()).cast() }
}

unsafe impl<T> Storage for SingleStackStorage<T> {
    type Handle = ();

    #[inline]
    unsafe fn get(&self, _: Self::Handle) -> NonNull<u8> { self.shared_get_mut(()) }

    #[inline]
    unsafe fn get_mut(&mut self, _: Self::Handle) -> NonNull<u8> { self.shared_get_mut(()) }

    #[inline]
    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        if !*self.allocated.get_mut() && Self::fits(layout.into()) {
            *self.allocated.get_mut() = true;
            Ok(NonEmptyMemoryBlock {
                size: unsafe { NonZeroUsize::new_unchecked(mem::size_of::<T>()) },
                handle: (),
            })
        } else {
            Err(AllocErr::new(layout.into()))
        }
    }

    #[inline]
    fn allocate(&mut self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if !*self.allocated.get_mut() && Self::fits(layout) {
            *self.allocated.get_mut() |= layout.size() != 0;
            Ok(MemoryBlock {
                size: mem::size_of::<T>(),
                handle: (),
            })
        } else {
            Err(AllocErr::new(layout))
        }
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, _: Self::Handle, _: NonEmptyLayout) { *self.allocated.get_mut() = false; }

    #[inline]
    unsafe fn deallocate(&mut self, _: Self::Handle, layout: Layout) {
        *self.allocated.get_mut() &= layout.size() == 0;
    }
}

unsafe impl<T> SharedStorage for SingleStackStorage<T> {
    #[inline]
    fn shared_allocate_nonempty(&self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        if Self::fits(layout.into()) && self.aquire() {
            Ok(NonEmptyMemoryBlock {
                size: unsafe { NonZeroUsize::new_unchecked(mem::size_of::<T>()) },
                handle: (),
            })
        } else {
            Err(AllocErr::new(layout.into()))
        }
    }

    #[inline]
    fn shared_allocate(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if Self::fits(layout) && (layout.size() == 0 || self.aquire()) {
            Ok(MemoryBlock {
                size: mem::size_of::<T>(),
                handle: (),
            })
        } else {
            Err(AllocErr::new(layout))
        }
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, _: Self::Handle, _: NonEmptyLayout) {
        self.allocated.store(false, Ordering::Release);
    }

    #[inline]
    unsafe fn shared_deallocate(&self, _: Self::Handle, layout: Layout) {
        self.allocated.fetch_and(layout.size() == 0, Ordering::Release);
    }
}

unsafe impl<T> SharedGetMut for OffsetSingleStackStorage<T> {
    unsafe fn shared_get_mut(&self, _: Self::Handle) -> NonNull<u8> { self.get(()) }
}

unsafe impl<T> OffsetHandle for OffsetSingleStackStorage<T> {
    unsafe fn offset(&mut self, _: Self::Handle, offset: isize) -> Self::Handle { self.offset.get().write(offset) }
}

unsafe impl<T> SharedOffsetHandle for OffsetSingleStackStorage<T> {
    unsafe fn shared_offset(&self, _: Self::Handle, offset: isize) -> Self::Handle { self.offset.get().write(offset) }
}

unsafe impl<T> Storage for OffsetSingleStackStorage<T> {
    type Handle = ();

    #[inline]
    unsafe fn get(&self, _: Self::Handle) -> NonNull<u8> {
        NonNull::new_unchecked(
            self.storage
                .memory
                .get()
                .cast::<u8>()
                .offset(self.offset.get().read())
                .cast::<T>(),
        )
        .cast()
    }

    #[inline]
    unsafe fn get_mut(&mut self, _: Self::Handle) -> NonNull<u8> { self.get(()) }

    #[inline]
    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.storage.allocate_nonempty(layout)
    }

    #[inline]
    fn allocate(&mut self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.allocate(layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout) {
        self.storage.deallocate_nonempty(handle, layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout) { self.storage.deallocate(handle, layout) }
}

unsafe impl<T> SharedStorage for OffsetSingleStackStorage<T> {
    #[inline]
    fn shared_allocate_nonempty(&self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_allocate_nonempty(layout)
    }

    #[inline]
    fn shared_allocate(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_allocate(layout)
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout) {
        self.storage.shared_deallocate_nonempty(handle, layout)
    }

    #[inline]
    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: Layout) {
        self.storage.shared_deallocate(handle, layout)
    }
}

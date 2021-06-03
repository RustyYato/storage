use core::{alloc::Layout, marker, mem, mem::MaybeUninit, num::NonZeroUsize, pin::Pin, ptr::NonNull};

use crate::{
    AllocErr, Handle, MemoryBlock, MultiStorage, NonEmptyLayout, NonEmptyMemoryBlock, ResizableStorage, SharedGetMut,
    Storage,
};

#[must_use = "storages don't do anything unless they are used"]
pub struct MultiStackStorage<T> {
    storage: MaybeUninit<T>,
    offset: usize,
    _pinned: marker::PhantomPinned,
}

unsafe impl<T> Send for MultiStackStorage<T> {}
unsafe impl<T> Sync for MultiStackStorage<T> {}

impl<T> MultiStackStorage<T> {
    pub const fn new() -> Self {
        Self {
            storage: MaybeUninit::uninit(),
            offset: mem::size_of::<T>(),
            _pinned: marker::PhantomPinned,
        }
    }
}

#[derive(Clone, Copy)]
pub struct MultiHandle(usize);

unsafe impl Handle for MultiHandle {
    unsafe fn dangling(_: usize) -> Self { Self(usize::MAX) }
}

impl MultiHandle {
    #[must_use = "`MultiHandle::is_dangling` should be used"]
    pub const fn is_dangling(self) -> bool { self.0 == usize::MAX }
}

unsafe impl<T> SharedGetMut for MultiStackStorage<T> {
    unsafe fn shared_get_mut(&self, MultiHandle(offset): Self::Handle) -> NonNull<u8> {
        let ptr = self.storage.as_ptr().cast::<u8>() as *mut u8;
        NonNull::new_unchecked(ptr.add(offset))
    }
}

impl<T> MultiStorage for MultiStackStorage<T> {}

unsafe impl<T> Storage for MultiStackStorage<T> {
    type Handle = MultiHandle;

    unsafe fn get(&self, MultiHandle(offset): Self::Handle) -> NonNull<u8> {
        let ptr = self.storage.as_ptr().cast::<u8>() as *mut u8;
        NonNull::new_unchecked(ptr.add(offset))
    }

    unsafe fn get_mut(&mut self, MultiHandle(offset): Self::Handle) -> NonNull<u8> {
        let ptr = self.storage.as_mut_ptr().cast::<u8>();
        NonNull::new_unchecked(ptr.add(offset))
    }

    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let layout = Layout::from(layout);

        // this is necessary so that the storage can be moved
        // between allocation and getting the pointer, otherwise
        // we would have to allocate more space than necessary
        // and offset the pointer each time to the correct alignment
        // but this is more expensive, and could be layered on top
        // if necessary
        if mem::align_of::<T>() < layout.align() {
            return Err(AllocErr(layout))
        }

        let begin = self.offset.checked_sub(layout.size()).ok_or(AllocErr(layout))?;
        let begin = begin & !layout.align().wrapping_sub(1);
        let size = unsafe { NonZeroUsize::new_unchecked(self.offset.wrapping_sub(begin)) };
        self.offset = begin;

        Ok(NonEmptyMemoryBlock {
            handle: MultiHandle(begin),
            size,
        })
    }

    unsafe fn deallocate_nonempty(&mut self, _: Self::Handle, _: NonEmptyLayout) {}
}

unsafe impl<T> ResizableStorage for MultiStackStorage<T> {
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if old == new {
            Ok(MemoryBlock {
                size: old.size(),
                handle,
            })
        } else {
            crate::defaults::grow(self, handle, old, new)
        }
    }

    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if old == new {
            Ok(MemoryBlock {
                size: old.size(),
                handle,
            })
        } else {
            crate::defaults::grow_zeroed(self, handle, old, new)
        }
    }

    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if old == new {
            Ok(MemoryBlock {
                size: old.size(),
                handle,
            })
        } else {
            crate::defaults::shrink(self, handle, old, new)
        }
    }
}

unsafe impl<T> SharedGetMut for Pin<&mut MultiStackStorage<T>> {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> {
        let this = Pin::get_ref(self.as_ref());
        this.shared_get_mut(handle)
    }
}

impl<T> MultiStorage for Pin<&mut MultiStackStorage<T>> {}

unsafe impl<T> Storage for Pin<&mut MultiStackStorage<T>> {
    type Handle = MultiHandle;

    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> {
        let this = Pin::get_ref(self.as_ref());
        this.get(handle)
    }

    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> {
        let this = Pin::get_unchecked_mut(self.as_mut());
        this.get_mut(handle)
    }

    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let this = unsafe { Pin::get_unchecked_mut(self.as_mut()) };

        let layout = Layout::from(layout);

        let begin = this.offset.checked_sub(layout.size()).ok_or(AllocErr(layout))?;
        let begin = begin & !layout.align().wrapping_sub(1);
        let size = unsafe { NonZeroUsize::new_unchecked(this.offset.wrapping_sub(begin)) };
        this.offset = begin;

        Ok(NonEmptyMemoryBlock {
            handle: MultiHandle(begin),
            size,
        })
    }

    unsafe fn deallocate_nonempty(&mut self, _: Self::Handle, _: NonEmptyLayout) {}
}

unsafe impl<T> ResizableStorage for Pin<&mut MultiStackStorage<T>> {
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if old == new {
            Ok(MemoryBlock {
                size: old.size(),
                handle,
            })
        } else {
            crate::defaults::grow(self, handle, old, new)
        }
    }

    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if old == new {
            Ok(MemoryBlock {
                size: old.size(),
                handle,
            })
        } else {
            crate::defaults::grow_zeroed(self, handle, old, new)
        }
    }

    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if old == new {
            Ok(MemoryBlock {
                size: old.size(),
                handle,
            })
        } else {
            crate::defaults::shrink(self, handle, old, new)
        }
    }
}

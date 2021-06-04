use core::{
    alloc::Layout,
    num::NonZeroUsize,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    AllocErr, FromPtr, Handle, MemoryBlock, MultiStorage, NonEmptyLayout, NonEmptyMemoryBlock, OffsetHandle,
    ResizableStorage, SharedGetMut, SharedOffsetHandle, SharedResizableStorage, SharedStorage, Storage,
};

#[must_use = "storages don't do anything unless they are used"]
pub struct BumpStorage<S: Storage, const MAX_ALIGN: usize> {
    storage: S,
    start: S::Handle,
    offset: AtomicUsize,
}

impl<S: Storage, const MAX_ALIGN: usize> BumpStorage<S, MAX_ALIGN> {
    const MAX_ALIGN_POW2: usize = MAX_ALIGN.next_power_of_two();

    pub fn new(storage: S, space: usize) -> Self { Self::try_new(storage, space).unwrap_or_else(AllocErr::handle) }

    pub fn remaining_space(&self) -> usize { self.offset.load(Ordering::Relaxed) }

    /// # Panics
    ///
    /// if `Layout::from_size_align(space, MAX_ALIGN.next_power_of_two())` panics
    pub fn try_new(mut storage: S, space: usize) -> Result<Self, AllocErr> {
        let memory_block = storage.allocate(Layout::from_size_align(space, Self::MAX_ALIGN_POW2).unwrap())?;
        Ok(Self {
            start: memory_block.handle,
            offset: AtomicUsize::new(memory_block.size),
            storage,
        })
    }
}

#[derive(Clone, Copy)]
pub struct BumpHandle(usize);

unsafe impl Handle for BumpHandle {
    unsafe fn dangling(_: usize) -> Self { Self(usize::MAX) }
}

impl BumpHandle {
    #[must_use = "`MultiHandle::is_dangling` should be used"]
    pub const fn is_dangling(self) -> bool { self.0 == usize::MAX }
}

unsafe impl<S: Storage, const MAX_ALIGN: usize> OffsetHandle for BumpStorage<S, MAX_ALIGN> {
    unsafe fn offset(&mut self, BumpHandle(handle): Self::Handle, offset: isize) -> Self::Handle {
        let offset = offset.to_ne_bytes();
        let offset = usize::from_ne_bytes(offset);
        BumpHandle(handle.wrapping_add(offset))
    }
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> SharedOffsetHandle for BumpStorage<S, MAX_ALIGN> {
    unsafe fn shared_offset(&self, BumpHandle(handle): Self::Handle, offset: isize) -> Self::Handle {
        let offset = offset.to_ne_bytes();
        let offset = usize::from_ne_bytes(offset);
        BumpHandle(handle.wrapping_add(offset))
    }
}

unsafe impl<S: Storage, const MAX_ALIGN: usize> FromPtr for BumpStorage<S, MAX_ALIGN> {
    #[allow(clippy::cast_sign_loss)]
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle {
        let origin = self.storage.get(self.start);
        BumpHandle(ptr.as_ptr().offset_from(origin.as_ptr()) as usize)
    }
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> SharedGetMut for BumpStorage<S, MAX_ALIGN> {
    unsafe fn shared_get_mut(&self, BumpHandle(offset): Self::Handle) -> NonNull<u8> {
        let ptr = self.storage.shared_get_mut(self.start);
        NonNull::new_unchecked(ptr.as_ptr().add(offset))
    }
}

impl<S: SharedGetMut, const MAX_ALIGN: usize> MultiStorage for BumpStorage<S, MAX_ALIGN> {}

unsafe impl<S: Storage, const MAX_ALIGN: usize> Storage for BumpStorage<S, MAX_ALIGN> {
    type Handle = BumpHandle;

    unsafe fn get(&self, BumpHandle(offset): Self::Handle) -> NonNull<u8> {
        let ptr = self.storage.get(self.start);
        NonNull::new_unchecked(ptr.as_ptr().add(offset))
    }

    unsafe fn get_mut(&mut self, BumpHandle(offset): Self::Handle) -> NonNull<u8> {
        let ptr = self.storage.get_mut(self.start);
        NonNull::new_unchecked(ptr.as_ptr().add(offset))
    }

    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let layout = Layout::from(layout);

        // this is necessary so that the storage can be moved
        // between allocation and getting the pointer, otherwise
        // we would have to allocate more space than necessary
        // and offset the pointer each time to the correct alignment
        // but this is more expensive, and could be layered on top
        // if necessary
        if Self::MAX_ALIGN_POW2 < layout.align() {
            return Err(AllocErr::new(layout))
        }

        let start = *self.offset.get_mut();

        let offset = start.checked_sub(layout.size()).ok_or_else(|| AllocErr::new(layout))?;
        let offset = offset & !layout.align().wrapping_sub(1);
        *self.offset.get_mut() = offset;

        let size = unsafe { NonZeroUsize::new_unchecked(start.wrapping_sub(offset)) };

        Ok(NonEmptyMemoryBlock {
            handle: BumpHandle(offset),
            size,
        })
    }

    unsafe fn deallocate_nonempty(&mut self, _: Self::Handle, _: NonEmptyLayout) {}
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> ResizableStorage for BumpStorage<S, MAX_ALIGN> {
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

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> SharedStorage for BumpStorage<S, MAX_ALIGN> {
    fn shared_allocate_nonempty(&self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let layout = Layout::from(layout);

        // this is necessary so that the storage can be moved
        // between allocation and getting the pointer, otherwise
        // we would have to allocate more space than necessary
        // and offset the pointer each time to the correct alignment
        // but this is more expensive, and could be layered on top
        // if necessary
        if Self::MAX_ALIGN_POW2 < layout.align() {
            return Err(AllocErr::new(layout))
        }

        let mut start = 0;
        let mut end = 0;
        self.offset
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |offset| {
                start = offset;

                let offset = offset.checked_sub(layout.size())?;
                let offset = offset & !layout.align().wrapping_sub(1);
                end = offset;

                Some(offset)
            })
            .map_err(|_| AllocErr::new(layout))?;
        let offset = end;

        let size = unsafe { NonZeroUsize::new_unchecked(start.wrapping_sub(offset)) };

        Ok(NonEmptyMemoryBlock {
            handle: BumpHandle(offset),
            size,
        })
    }

    unsafe fn shared_deallocate_nonempty(&self, _: Self::Handle, _: NonEmptyLayout) {}
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> SharedResizableStorage for BumpStorage<S, MAX_ALIGN> {
    unsafe fn shared_grow(
        &self,
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

    unsafe fn shared_grow_zeroed(
        &self,
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

    unsafe fn shared_shrink(
        &self,
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

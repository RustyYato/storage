use core::{
    alloc::Layout,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    AllocErr, BumpHandle, BumpStorage, FromPtr, MemoryBlock, MultiStorage, NonEmptyLayout, NonEmptyMemoryBlock,
    OffsetHandle, ResizableStorage, SharedGetMut, SharedOffsetHandle, SharedResizableStorage, SharedStorage, Storage,
};

#[must_use = "storages don't do anything unless they are used"]
pub struct CountingBumpStorage<S: Storage, const MAX_ALIGN: usize> {
    bump: BumpStorage<S, MAX_ALIGN>,
    max_offset: usize,
    count: AtomicUsize,
}

impl<S: Storage, const MAX_ALIGN: usize> CountingBumpStorage<S, MAX_ALIGN> {
    pub fn new(storage: S, space: usize) -> Self { Self::try_new(storage, space).unwrap_or_else(AllocErr::handle) }

    pub fn remaining_space(&self) -> usize { self.bump.remaining_space() }

    /// # Panics
    ///
    /// if `Layout::from_size_align(space, MAX_ALIGN.next_power_of_two())` returns Err
    pub fn try_new(storage: S, space: usize) -> Result<Self, AllocErr> {
        let bump = BumpStorage::try_new(storage, space)?;
        Ok(Self {
            count: AtomicUsize::new(0),
            max_offset: bump.remaining_space(),
            bump,
        })
    }
}

unsafe impl<S: Storage, const MAX_ALIGN: usize> OffsetHandle for CountingBumpStorage<S, MAX_ALIGN> {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        self.bump.offset(handle, offset)
    }
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> SharedOffsetHandle for CountingBumpStorage<S, MAX_ALIGN> {
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        self.bump.shared_offset(handle, offset)
    }
}

unsafe impl<S: Storage, const MAX_ALIGN: usize> FromPtr for CountingBumpStorage<S, MAX_ALIGN> {
    #[allow(clippy::cast_sign_loss)]
    unsafe fn from_ptr(&self, ptr: NonNull<u8>, layout: Layout) -> Self::Handle { self.bump.from_ptr(ptr, layout) }
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> SharedGetMut for CountingBumpStorage<S, MAX_ALIGN> {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { self.bump.shared_get_mut(handle) }
}

impl<S: SharedGetMut, const MAX_ALIGN: usize> MultiStorage for CountingBumpStorage<S, MAX_ALIGN> {}

unsafe impl<S: Storage, const MAX_ALIGN: usize> Storage for CountingBumpStorage<S, MAX_ALIGN> {
    type Handle = BumpHandle;

    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { self.bump.get(handle) }

    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { self.bump.get_mut(handle) }

    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.bump.allocate_nonempty(layout)?;
        *self.count.get_mut() += 1;
        Ok(memory_block)
    }

    unsafe fn deallocate_nonempty(&mut self, _: Self::Handle, _: NonEmptyLayout) {
        let count = self.count.get_mut();
        *count -= 1;
        if *count == 0 {
            self.bump.reset(self.max_offset)
        }
    }
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> ResizableStorage for CountingBumpStorage<S, MAX_ALIGN> {
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.bump.grow(handle, old, new)
    }

    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.bump.grow_zeroed(handle, old, new)
    }

    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.bump.shrink(handle, old, new)
    }
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> SharedStorage for CountingBumpStorage<S, MAX_ALIGN> {
    fn shared_allocate_nonempty(&self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.bump.shared_allocate_nonempty(layout)?;
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(memory_block)
    }

    unsafe fn shared_deallocate_nonempty(&self, _: Self::Handle, _: NonEmptyLayout) {
        let current_offset = self.bump.remaining_space();
        if 1 == self.count.fetch_sub(1, Ordering::Relaxed) {
            self.bump.shared_reset_if_eq(current_offset, self.max_offset);
        }
    }
}

unsafe impl<S: SharedGetMut, const MAX_ALIGN: usize> SharedResizableStorage for CountingBumpStorage<S, MAX_ALIGN> {
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.bump.shared_grow(handle, old, new)
    }

    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.bump.shared_grow_zeroed(handle, old, new)
    }

    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.bump.shared_shrink(handle, old, new)
    }
}

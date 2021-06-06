use core::{alloc::Layout, ptr::NonNull};

use crate::{
    AllocErr, Flush, FromPtr, MemoryBlock, MultiStorage, NonEmptyLayout, NonEmptyMemoryBlock, OffsetHandle,
    ResizableStorage, SharedFlush, SharedGetMut, SharedOffsetHandle, SharedResizableStorage, SharedStorage, Storage,
};

#[must_use = "storages don't do anything unless they are used"]
pub struct FlushBarrier<S> {
    pub storage: S,
}

impl<S> FlushBarrier<S> {
    pub const fn new(storage: S) -> Self { Self { storage } }
}

impl<S> Flush for FlushBarrier<S> {
    fn try_flush(&mut self) -> bool { true }

    fn flush(&mut self) {}
}

impl<S> SharedFlush for FlushBarrier<S> {
    fn try_shared_flush(&self) -> bool { true }

    fn shared_flush(&self) {}
}

unsafe impl<S: OffsetHandle> OffsetHandle for FlushBarrier<S> {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        self.storage.offset(handle, offset)
    }
}

unsafe impl<S: SharedOffsetHandle> SharedOffsetHandle for FlushBarrier<S> {
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        self.storage.shared_offset(handle, offset)
    }
}

unsafe impl<S: FromPtr> FromPtr for FlushBarrier<S> {
    #[allow(clippy::cast_sign_loss)]
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle { self.storage.from_ptr(ptr) }
}

unsafe impl<S: SharedGetMut> SharedGetMut for FlushBarrier<S> {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { self.storage.shared_get_mut(handle) }
}

impl<S: MultiStorage> MultiStorage for FlushBarrier<S> {}

unsafe impl<S: Storage> Storage for FlushBarrier<S> {
    type Handle = S::Handle;

    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { self.storage.get(handle) }

    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { self.storage.get_mut(handle) }

    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.storage.allocate_nonempty(layout)
    }

    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout) {
        self.storage.deallocate_nonempty(handle, layout);
    }

    fn allocate(&mut self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.allocate(layout)
    }

    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout) { self.storage.deallocate(handle, layout); }

    fn allocate_nonempty_zeroed(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.storage.allocate_nonempty_zeroed(layout)
    }

    fn allocate_zeroed(&mut self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.allocate_zeroed(layout)
    }
}

unsafe impl<S: ResizableStorage> ResizableStorage for FlushBarrier<S> {
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.grow(handle, old, new)
    }

    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.grow_zeroed(handle, old, new)
    }

    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shrink(handle, old, new)
    }
}

unsafe impl<S: SharedStorage> SharedStorage for FlushBarrier<S> {
    fn shared_allocate_nonempty(&self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_allocate_nonempty(layout)
    }

    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout) {
        self.storage.shared_deallocate_nonempty(handle, layout);
    }

    fn shared_allocate(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_allocate(layout)
    }

    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: Layout) {
        self.storage.shared_deallocate(handle, layout);
    }

    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_allocate_nonempty_zeroed(layout)
    }

    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_allocate_zeroed(layout)
    }
}

unsafe impl<S: SharedResizableStorage> SharedResizableStorage for FlushBarrier<S> {
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_grow(handle, old, new)
    }

    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_grow_zeroed(handle, old, new)
    }

    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_shrink(handle, old, new)
    }
}

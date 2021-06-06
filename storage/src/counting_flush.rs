use core::{
    alloc::Layout,
    ptr::NonNull,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{
    AllocErr, Flush, FromPtr, MemoryBlock, MultiStorage, NonEmptyLayout, NonEmptyMemoryBlock, OffsetHandle,
    ResizableStorage, SharedFlush, SharedGetMut, SharedOffsetHandle, SharedResizableStorage, SharedStorage, Storage,
};

const THRESHOLD: u8 = 128;

#[must_use = "storages don't do anything unless they are used"]
pub struct CountingFlushStorage<S> {
    pub storage: S,
    count: AtomicU8,
}

impl<S: Storage + Flush> CountingFlushStorage<S> {
    #[inline]
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            count: AtomicU8::new(0),
        }
    }

    #[cold]
    #[inline(never)]
    fn flush_slow(&mut self) { self.storage.flush() }

    #[cold]
    #[inline(never)]
    fn shared_flush_slow(&self)
    where
        S: SharedFlush,
    {
        self.storage.shared_flush()
    }

    #[inline]
    fn count(&mut self) {
        let count = self.count.get_mut();
        if *count > THRESHOLD {
            *count = 0;
            self.flush_slow()
        } else {
            *count += 1;
        }
    }

    #[inline]
    fn shared_count(&self)
    where
        S: SharedFlush,
    {
        if self.count.fetch_add(1, Ordering::Relaxed) > THRESHOLD {
            self.count.fetch_sub(THRESHOLD, Ordering::Relaxed);
            self.shared_flush_slow()
        }
    }
}

impl<S: Flush> Flush for CountingFlushStorage<S> {
    #[inline]
    fn try_flush(&mut self) -> bool {
        *self.count.get_mut() = 0;
        self.storage.try_flush()
    }

    #[inline]
    fn flush(&mut self) {
        *self.count.get_mut() = 0;
        self.storage.flush();
    }
}

impl<S: SharedFlush> SharedFlush for CountingFlushStorage<S> {
    #[inline]
    fn try_shared_flush(&self) -> bool {
        self.count.store(0, Ordering::Relaxed);
        self.storage.try_shared_flush()
    }

    #[inline]
    fn shared_flush(&self) {
        self.count.store(0, Ordering::Relaxed);
        self.storage.shared_flush();
    }
}

unsafe impl<S: OffsetHandle + Flush> OffsetHandle for CountingFlushStorage<S> {
    #[inline]
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        self.storage.offset(handle, offset)
    }
}

unsafe impl<S: SharedOffsetHandle + SharedFlush> SharedOffsetHandle for CountingFlushStorage<S> {
    #[inline]
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        self.storage.shared_offset(handle, offset)
    }
}

unsafe impl<S: FromPtr + Flush> FromPtr for CountingFlushStorage<S> {
    #[inline]
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle { self.storage.from_ptr(ptr) }
}

unsafe impl<S: SharedGetMut + Flush> SharedGetMut for CountingFlushStorage<S> {
    #[inline]
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { self.storage.shared_get_mut(handle) }
}

impl<S: MultiStorage + Flush> MultiStorage for CountingFlushStorage<S> {}

unsafe impl<S: Storage + Flush> Storage for CountingFlushStorage<S> {
    type Handle = S::Handle;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { self.storage.get(handle) }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { self.storage.get_mut(handle) }

    #[inline]
    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.count();
        self.storage.allocate_nonempty(layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout) {
        self.storage.deallocate_nonempty(handle, layout);
        self.count();
    }

    #[inline]
    fn allocate(&mut self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.count();
        self.storage.allocate(layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout) {
        self.storage.deallocate(handle, layout);
        self.count();
    }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.count();
        self.storage.allocate_nonempty_zeroed(layout)
    }

    #[inline]
    fn allocate_zeroed(&mut self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.count();
        self.storage.allocate_zeroed(layout)
    }
}

unsafe impl<S: ResizableStorage + Flush> ResizableStorage for CountingFlushStorage<S> {
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.storage.grow(handle, old, new);
        self.count();
        memory_block
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.storage.grow_zeroed(handle, old, new);
        self.count();
        memory_block
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.storage.shrink(handle, old, new);
        self.count();
        memory_block
    }
}

unsafe impl<S: SharedStorage + SharedFlush> SharedStorage for CountingFlushStorage<S> {
    #[inline]
    fn shared_allocate_nonempty(&self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.shared_count();
        self.storage.shared_allocate_nonempty(layout)
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout) {
        self.storage.shared_deallocate_nonempty(handle, layout);
        self.shared_count();
    }

    #[inline]
    fn shared_allocate(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.shared_count();
        self.storage.shared_allocate(layout)
    }

    #[inline]
    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: Layout) {
        self.storage.shared_deallocate(handle, layout);
        self.shared_count();
    }

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        self.shared_count();
        self.storage.shared_allocate_nonempty_zeroed(layout)
    }

    #[inline]
    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        self.shared_count();
        self.storage.shared_allocate_zeroed(layout)
    }
}

unsafe impl<S: SharedResizableStorage + SharedFlush> SharedResizableStorage for CountingFlushStorage<S> {
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.storage.shared_grow(handle, old, new);
        self.shared_count();
        memory_block
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.storage.shared_grow_zeroed(handle, old, new);
        self.shared_count();
        memory_block
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        let memory_block = self.storage.shared_shrink(handle, old, new);
        self.shared_count();
        memory_block
    }
}

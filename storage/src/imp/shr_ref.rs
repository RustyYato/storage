use core::{alloc::Layout, ptr::NonNull};

use crate::{
    Flush, FromPtr, MultiStorage, OffsetHandle, ResizableStorage, SharedFlush, SharedGetMut, SharedOffsetHandle,
    SharedResizableStorage, SharedStorage, Storage,
};

impl<S: SharedFlush + ?Sized> Flush for &S {
    fn try_flush(&mut self) -> bool { S::try_shared_flush(self) }

    fn flush(&mut self) { S::shared_flush(self) }
}

impl<S: SharedFlush + ?Sized> SharedFlush for &S {
    fn try_shared_flush(&self) -> bool { S::try_shared_flush(self) }

    fn shared_flush(&self) { S::shared_flush(self) }
}

unsafe impl<S: FromPtr + SharedStorage + ?Sized> FromPtr for &S {
    #[inline]
    unsafe fn from_ptr(&self, ptr: NonNull<u8>, layout: Layout) -> Self::Handle { S::from_ptr(self, ptr, layout) }

    #[inline]
    unsafe fn from_ptr_mut(&mut self, ptr: NonNull<u8>, layout: Layout) -> Self::Handle {
        S::from_ptr(self, ptr, layout)
    }
}

unsafe impl<S: SharedOffsetHandle + ?Sized> OffsetHandle for &S {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        S::shared_offset(self, handle, offset)
    }
}

unsafe impl<S: SharedOffsetHandle + ?Sized> SharedOffsetHandle for &S {
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        S::shared_offset(self, handle, offset)
    }
}

impl<S: MultiStorage + SharedStorage + ?Sized> MultiStorage for &S {}

unsafe impl<S: SharedStorage + ?Sized> Storage for &S {
    type Handle = S::Handle;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { S::get(self, handle) }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { S::shared_get_mut(self, handle) }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate_nonempty(self, layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        S::shared_deallocate_nonempty(self, handle, layout)
    }

    #[inline]
    fn allocate(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate(self, layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout) { S::shared_deallocate(self, handle, layout) }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate_nonempty_zeroed(self, layout)
    }

    #[inline]
    fn allocate_zeroed(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate_zeroed(self, layout)
    }
}

unsafe impl<S: SharedGetMut + SharedStorage + ?Sized> SharedGetMut for &S {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { S::shared_get_mut(self, handle) }
}

unsafe impl<S: SharedResizableStorage + ?Sized> ResizableStorage for &S {
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_grow(self, handle, old, new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_grow_zeroed(self, handle, old, new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_shrink(self, handle, old, new)
    }
}

unsafe impl<S: SharedStorage + ?Sized> SharedStorage for &S {
    #[inline]
    fn shared_allocate_nonempty(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate_nonempty(self, layout)
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        S::shared_deallocate_nonempty(self, handle, layout)
    }

    #[inline]
    fn shared_allocate(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate(self, layout)
    }

    #[inline]
    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: Layout) {
        S::shared_deallocate(self, handle, layout)
    }

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate_nonempty_zeroed(self, layout)
    }

    #[inline]
    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate_zeroed(self, layout)
    }
}

unsafe impl<S: SharedResizableStorage + ?Sized> SharedResizableStorage for &S {
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_grow(self, handle, old, new)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_grow_zeroed(self, handle, old, new)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_shrink(self, handle, old, new)
    }
}

use crate::{
    Flush, FromPtr, MultiStorage, OffsetHandle, ResizableStorage, SharedFlush, SharedGetMut, SharedOffsetHandle,
    SharedResizableStorage, SharedStorage, Storage,
};
use core::ptr::NonNull;

impl<S: Flush + ?Sized> Flush for &mut S {
    fn try_flush(&mut self) -> bool { S::try_flush(self) }

    fn flush(&mut self) { S::flush(self) }
}

impl<S: SharedFlush + ?Sized> SharedFlush for &mut S {
    fn try_shared_flush(&self) -> bool { S::try_shared_flush(self) }

    fn shared_flush(&self) { S::shared_flush(self) }
}

unsafe impl<S: FromPtr + ?Sized> FromPtr for &mut S {
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle { S::from_ptr(self, ptr) }
}

unsafe impl<S: OffsetHandle + ?Sized> OffsetHandle for &mut S {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle { S::offset(self, handle, offset) }
}

unsafe impl<S: SharedOffsetHandle + ?Sized> SharedOffsetHandle for &mut S {
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        S::shared_offset(self, handle, offset)
    }
}

impl<S: MultiStorage + ?Sized> MultiStorage for &mut S {}
unsafe impl<S: Storage + ?Sized> Storage for &mut S {
    type Handle = S::Handle;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { S::get(self, handle) }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { S::get_mut(self, handle) }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        S::allocate_nonempty(self, layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        S::deallocate_nonempty(self, handle, layout)
    }

    #[inline]
    fn allocate(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::allocate(self, layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: core::alloc::Layout) {
        S::deallocate(self, handle, layout)
    }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        S::allocate_nonempty_zeroed(self, layout)
    }

    #[inline]
    fn allocate_zeroed(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::allocate_zeroed(self, layout)
    }
}

unsafe impl<S: SharedGetMut + ?Sized> SharedGetMut for &mut S {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { S::shared_get_mut(self, handle) }
}

unsafe impl<S: ResizableStorage + ?Sized> ResizableStorage for &mut S {
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::grow(self, handle, old, new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::grow_zeroed(self, handle, old, new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shrink(self, handle, old, new)
    }
}

unsafe impl<S: SharedStorage + ?Sized> SharedStorage for &mut S {
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
    fn shared_allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate(self, layout)
    }

    #[inline]
    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: core::alloc::Layout) {
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
    fn shared_allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_allocate_zeroed(self, layout)
    }
}

unsafe impl<S: SharedResizableStorage + ?Sized> SharedResizableStorage for &mut S {
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_grow(self, handle, old, new)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_grow_zeroed(self, handle, old, new)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        S::shared_shrink(self, handle, old, new)
    }
}

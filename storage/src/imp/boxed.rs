use crate::{
    boxed::Box, Flush, FromPtr, MultiStorage, OffsetHandle, ResizableStorage, SharedFlush, SharedGetMut,
    SharedOffsetHandle, SharedResizableStorage, SharedStorage, Storage,
};
use core::{alloc::Layout, ptr::NonNull};

impl<T: Flush + ?Sized, S: Storage> Flush for Box<T, S> {
    fn try_flush(&mut self) -> bool { T::try_flush(self) }

    fn flush(&mut self) { T::flush(self) }
}

impl<T: SharedFlush + ?Sized, S: Storage> SharedFlush for Box<T, S> {
    fn try_shared_flush(&self) -> bool { T::try_shared_flush(self) }

    fn shared_flush(&self) { T::shared_flush(self) }
}

unsafe impl<T: FromPtr + ?Sized, S: Storage> FromPtr for Box<T, S> {
    unsafe fn from_ptr(&self, ptr: NonNull<u8>, layout: Layout) -> Self::Handle { T::from_ptr(self, ptr, layout) }
}

unsafe impl<T: OffsetHandle + ?Sized, S: Storage> OffsetHandle for Box<T, S> {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle { T::offset(self, handle, offset) }
}

unsafe impl<T: SharedOffsetHandle + ?Sized, S: Storage> SharedOffsetHandle for Box<T, S> {
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        T::shared_offset(self, handle, offset)
    }
}

impl<T: MultiStorage + ?Sized, S: Storage> MultiStorage for Box<T, S> {}
unsafe impl<T: Storage + ?Sized, S: Storage> Storage for Box<T, S> {
    type Handle = T::Handle;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { T::get(self, handle) }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { T::get_mut(self, handle) }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        T::allocate_nonempty(self, layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        T::deallocate_nonempty(self, handle, layout)
    }

    #[inline]
    fn allocate(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::allocate(self, layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: core::alloc::Layout) {
        T::deallocate(self, handle, layout)
    }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        T::allocate_nonempty_zeroed(self, layout)
    }

    #[inline]
    fn allocate_zeroed(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::allocate_zeroed(self, layout)
    }
}

unsafe impl<T: SharedGetMut + ?Sized, S: Storage> SharedGetMut for Box<T, S> {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { T::shared_get_mut(self, handle) }
}

unsafe impl<T: ResizableStorage + ?Sized, S: Storage> ResizableStorage for Box<T, S> {
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::grow(self, handle, old, new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::grow_zeroed(self, handle, old, new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shrink(self, handle, old, new)
    }
}

unsafe impl<T: SharedStorage + ?Sized, S: Storage> SharedStorage for Box<T, S> {
    #[inline]
    fn shared_allocate_nonempty(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_allocate_nonempty(self, layout)
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        T::shared_deallocate_nonempty(self, handle, layout)
    }

    #[inline]
    fn shared_allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_allocate(self, layout)
    }

    #[inline]
    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: core::alloc::Layout) {
        T::shared_deallocate(self, handle, layout)
    }

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_allocate_nonempty_zeroed(self, layout)
    }

    #[inline]
    fn shared_allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_allocate_zeroed(self, layout)
    }
}

unsafe impl<T: SharedResizableStorage + ?Sized, S: Storage> SharedResizableStorage for Box<T, S> {
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_grow(self, handle, old, new)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_grow_zeroed(self, handle, old, new)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_shrink(self, handle, old, new)
    }
}

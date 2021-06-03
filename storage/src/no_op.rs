use core::{alloc::Layout, ptr::NonNull};

use crate::{
    AllocErr, FromPtr, MultiStorage, ResizableStorage, SharedGetMut, SharedResizableStorage, SharedStorage, Storage,
};

pub struct NoOpStorage;

unsafe impl FromPtr for NoOpStorage {
    #[inline]
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle { ptr }
}

unsafe impl SharedGetMut for NoOpStorage {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { handle }
}

impl MultiStorage for NoOpStorage {}

unsafe impl Storage for NoOpStorage {
    type Handle = NonNull<u8>;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { handle }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { handle }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(layout.into()))
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, _: Self::Handle, _: crate::NonEmptyLayout) {}

    #[inline]
    fn allocate(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(layout))
    }

    #[inline]
    unsafe fn deallocate(&mut self, _: Self::Handle, _: Layout) {}

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(layout.into()))
    }

    #[inline]
    fn allocate_zeroed(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(layout))
    }
}

unsafe impl ResizableStorage for NoOpStorage {
    #[inline]
    unsafe fn grow(
        &mut self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(new))
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(new))
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(new))
    }
}

unsafe impl SharedStorage for NoOpStorage {
    #[inline]
    fn shared_allocate_nonempty(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(layout.into()))
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, _: Self::Handle, _: crate::NonEmptyLayout) {}

    #[inline]
    fn shared_allocate(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(layout))
    }

    #[inline]
    unsafe fn shared_deallocate(&self, _: Self::Handle, _: Layout) {}

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(layout.into()))
    }

    #[inline]
    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(layout))
    }
}

unsafe impl SharedResizableStorage for NoOpStorage {
    #[inline]
    unsafe fn shared_grow(
        &self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(new))
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(new))
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr(new))
    }
}

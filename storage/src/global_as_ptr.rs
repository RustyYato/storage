use crate::{
    affix::{OffsetHandle, SharedOffsetHandle},
    core_traits::FromPtr,
    macros::{map_mbr, map_nembr},
    MultiStorage, PointerHandle, ResizableStorage, SharedGetMut, SharedResizableStorage, SharedStorage, Storage,
};
use core::ptr::NonNull;

fn to_ptr<H: PointerHandle>(handle: H) -> NonNull<u8> { unsafe { handle.get_mut() } }

pub struct GlobalAsPtrStorage<S> {
    inner: S,
}

impl<S: 'static> GlobalAsPtrStorage<S> {
    pub const unsafe fn new(inner: S) -> Self { Self { inner } }
}

unsafe impl<S: FromPtr> FromPtr for GlobalAsPtrStorage<S>
where
    S::Handle: PointerHandle,
{
    #[inline]
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle { ptr }
}

unsafe impl<S: Storage + FromPtr> OffsetHandle for GlobalAsPtrStorage<S>
where
    S::Handle: PointerHandle,
{
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        NonNull::new_unchecked(handle.as_ptr().offset(offset))
    }
}

unsafe impl<S: SharedStorage + FromPtr> SharedOffsetHandle for GlobalAsPtrStorage<S>
where
    S::Handle: PointerHandle,
{
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        NonNull::new_unchecked(handle.as_ptr().offset(offset))
    }
}

impl<S: MultiStorage + FromPtr> MultiStorage for GlobalAsPtrStorage<S> where S::Handle: PointerHandle {}
unsafe impl<S: Storage + FromPtr> Storage for GlobalAsPtrStorage<S>
where
    S::Handle: PointerHandle,
{
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
        map_nembr(S::allocate_nonempty(&mut self.inner, layout), to_ptr)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        let handle = self.inner.from_ptr(handle);
        S::deallocate_nonempty(&mut self.inner, handle, layout)
    }

    #[inline]
    fn allocate(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        map_mbr(S::allocate(&mut self.inner, layout), to_ptr)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: core::alloc::Layout) {
        let handle = self.inner.from_ptr(handle);
        S::deallocate(&mut self.inner, handle, layout)
    }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        map_nembr(S::allocate_nonempty_zeroed(&mut self.inner, layout), to_ptr)
    }

    #[inline]
    fn allocate_zeroed(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        map_mbr(S::allocate_zeroed(&mut self.inner, layout), to_ptr)
    }
}

unsafe impl<S: SharedGetMut + FromPtr> SharedGetMut for GlobalAsPtrStorage<S>
where
    S::Handle: PointerHandle,
{
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { handle }
}

unsafe impl<S: ResizableStorage + FromPtr> ResizableStorage for GlobalAsPtrStorage<S>
where
    S::Handle: PointerHandle,
{
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let handle = self.inner.from_ptr(handle);
        map_mbr(S::grow(&mut self.inner, handle, old, new), to_ptr)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let handle = self.inner.from_ptr(handle);
        map_mbr(S::grow_zeroed(&mut self.inner, handle, old, new), to_ptr)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let handle = self.inner.from_ptr(handle);
        map_mbr(S::shrink(&mut self.inner, handle, old, new), to_ptr)
    }
}

unsafe impl<S: SharedStorage + FromPtr> SharedStorage for GlobalAsPtrStorage<S>
where
    S::Handle: PointerHandle,
{
    #[inline]
    fn shared_allocate_nonempty(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        map_nembr(S::shared_allocate_nonempty(&self.inner, layout), to_ptr)
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        let handle = self.inner.from_ptr(handle);
        S::shared_deallocate_nonempty(&self.inner, handle, layout)
    }

    #[inline]
    fn shared_allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        map_mbr(S::shared_allocate(&self.inner, layout), to_ptr)
    }

    #[inline]
    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: core::alloc::Layout) {
        let handle = self.inner.from_ptr(handle);
        S::shared_deallocate(&self.inner, handle, layout)
    }

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        map_nembr(S::shared_allocate_nonempty_zeroed(&self.inner, layout), to_ptr)
    }

    #[inline]
    fn shared_allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        map_mbr(S::shared_allocate_zeroed(&self.inner, layout), to_ptr)
    }
}

unsafe impl<S: SharedResizableStorage + FromPtr> SharedResizableStorage for GlobalAsPtrStorage<S>
where
    S::Handle: PointerHandle,
{
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let handle = self.inner.from_ptr(handle);
        map_mbr(S::shared_grow(&self.inner, handle, old, new), to_ptr)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let handle = self.inner.from_ptr(handle);
        map_mbr(S::shared_grow_zeroed(&self.inner, handle, old, new), to_ptr)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let handle = self.inner.from_ptr(handle);
        map_mbr(S::shared_shrink(&self.inner, handle, old, new), to_ptr)
    }
}

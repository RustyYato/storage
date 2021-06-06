use crate::{
    rc::{Counter, DynamicCounter, RefCounted, StrongKind},
    Flush, FromPtr, MultiStorage, OffsetHandle, ResizableStorage, SharedFlush, SharedGetMut, SharedOffsetHandle,
    SharedResizableStorage, SharedStorage, Storage,
};
use core::ptr::NonNull;

impl<T: SharedFlush + SharedStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> Flush
    for RefCounted<T, I, A, StrongKind, S>
{
    fn try_flush(&mut self) -> bool { T::try_shared_flush(self) }

    fn flush(&mut self) { T::shared_flush(self) }
}

impl<T: SharedFlush + SharedStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> SharedFlush
    for RefCounted<T, I, A, StrongKind, S>
{
    fn try_shared_flush(&self) -> bool { T::try_shared_flush(self) }

    fn shared_flush(&self) { T::shared_flush(self) }
}

unsafe impl<T: FromPtr + SharedStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> FromPtr
    for RefCounted<T, I, A, StrongKind, S>
{
    unsafe fn from_ptr(&self, ptr: NonNull<u8>) -> Self::Handle { T::from_ptr(self, ptr) }
}

unsafe impl<T: SharedOffsetHandle + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> OffsetHandle
    for RefCounted<T, I, A, StrongKind, S>
{
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        T::shared_offset(self, handle, offset)
    }
}

unsafe impl<T: SharedOffsetHandle + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> SharedOffsetHandle
    for RefCounted<T, I, A, StrongKind, S>
{
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        T::shared_offset(self, handle, offset)
    }
}

impl<T: MultiStorage + SharedStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> MultiStorage
    for RefCounted<T, I, A, StrongKind, S>
{
}

unsafe impl<T: SharedStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> Storage
    for RefCounted<T, I, A, StrongKind, S>
{
    type Handle = T::Handle;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { T::get(self, handle) }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { T::shared_get_mut(self, handle) }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_allocate_nonempty(self, layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        T::shared_deallocate_nonempty(self, handle, layout)
    }

    #[inline]
    fn allocate(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_allocate(self, layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: core::alloc::Layout) {
        T::shared_deallocate(self, handle, layout)
    }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_allocate_nonempty_zeroed(self, layout)
    }

    #[inline]
    fn allocate_zeroed(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_allocate_zeroed(self, layout)
    }
}

unsafe impl<T: SharedGetMut + SharedStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> SharedGetMut
    for RefCounted<T, I, A, StrongKind, S>
{
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { T::shared_get_mut(self, handle) }
}

unsafe impl<T: SharedResizableStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> ResizableStorage
    for RefCounted<T, I, A, StrongKind, S>
{
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_grow(self, handle, old, new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_grow_zeroed(self, handle, old, new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        T::shared_shrink(self, handle, old, new)
    }
}

unsafe impl<T: SharedStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> SharedStorage
    for RefCounted<T, I, A, StrongKind, S>
{
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

unsafe impl<T: SharedResizableStorage + ?Sized, I: DynamicCounter, A: Counter, S: OffsetHandle> SharedResizableStorage
    for RefCounted<T, I, A, StrongKind, S>
{
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

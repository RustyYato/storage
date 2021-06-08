use core::{alloc::Layout, marker::PhantomData, ptr::NonNull};

use crate::{
    AllocErr, Flush, FromPtr, Handle, ResizableStorage, SharedFlush, SharedGetMut, SharedResizableStorage,
    SharedStorage, Storage,
};

pub struct NullStorage<T = core::convert::Infallible>(PhantomData<T>);

impl<T> Flush for NullStorage<T> {
    fn try_flush(&mut self) -> bool { true }

    fn flush(&mut self) {}
}

impl<T> SharedFlush for NullStorage<T> {
    fn try_shared_flush(&self) -> bool { true }

    fn shared_flush(&self) {}
}

impl NullStorage {
    #[inline]
    pub const fn new() -> Self { Self::with_handle() }
}

impl<T> NullStorage<T> {
    #[inline]
    pub const fn with_handle() -> Self { Self(PhantomData) }
}

unsafe impl<H: Handle> FromPtr for NullStorage<H> {
    #[inline]
    unsafe fn from_ptr(&self, _: NonNull<u8>, _: Layout) -> Self::Handle { core::hint::unreachable_unchecked() }
}

unsafe impl<H: Handle> SharedGetMut for NullStorage<H> {
    #[inline]
    unsafe fn shared_get_mut(&self, _: Self::Handle) -> NonNull<u8> { core::hint::unreachable_unchecked() }
}

unsafe impl<H: Handle> Storage for NullStorage<H> {
    type Handle = H;

    #[inline]
    unsafe fn get(&self, _: Self::Handle) -> NonNull<u8> { core::hint::unreachable_unchecked() }

    #[inline]
    unsafe fn get_mut(&mut self, _: Self::Handle) -> NonNull<u8> { core::hint::unreachable_unchecked() }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout.into()))
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, _: Self::Handle, _: crate::NonEmptyLayout) {
        core::hint::unreachable_unchecked()
    }

    #[inline]
    fn allocate(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout))
    }

    #[inline]
    unsafe fn deallocate(&mut self, _: Self::Handle, _: Layout) { core::hint::unreachable_unchecked() }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout.into()))
    }

    #[inline]
    fn allocate_zeroed(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout))
    }
}

unsafe impl<H: Handle> ResizableStorage for NullStorage<H> {
    #[inline]
    unsafe fn grow(
        &mut self,
        _: Self::Handle,
        _: Layout,
        _: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        core::hint::unreachable_unchecked()
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        _: Self::Handle,
        _: Layout,
        _: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        core::hint::unreachable_unchecked()
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        _: Self::Handle,
        _: Layout,
        _: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        core::hint::unreachable_unchecked()
    }
}

unsafe impl<H: Handle> SharedStorage for NullStorage<H> {
    #[inline]
    fn shared_allocate_nonempty(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout.into()))
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, _: Self::Handle, _: crate::NonEmptyLayout) {
        core::hint::unreachable_unchecked()
    }

    #[inline]
    fn shared_allocate(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout))
    }

    #[inline]
    unsafe fn shared_deallocate(&self, _: Self::Handle, _: Layout) { core::hint::unreachable_unchecked() }

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout.into()))
    }

    #[inline]
    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout))
    }
}

unsafe impl<H: Handle> SharedResizableStorage for NullStorage<H> {
    #[inline]
    unsafe fn shared_grow(
        &self,
        _: Self::Handle,
        _: Layout,
        _: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        core::hint::unreachable_unchecked()
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        _: Self::Handle,
        _: Layout,
        _: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        core::hint::unreachable_unchecked()
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        _: Self::Handle,
        _: Layout,
        _: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        core::hint::unreachable_unchecked()
    }
}

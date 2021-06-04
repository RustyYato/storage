use core::{alloc::Layout, marker::PhantomData, ptr::NonNull};

use crate::{
    AllocErr, FromPtr, Handle, MemoryBlock, ResizableStorage, SharedGetMut, SharedResizableStorage, SharedStorage,
    Storage,
};

const MAX_ALIGN: usize = 1 << 29;
const DANGLING: NonNull<u8> = unsafe { NonNull::new_unchecked(MAX_ALIGN as *mut u8) };

pub struct ZeroSizedStorage<T>(PhantomData<T>);

impl<T> ZeroSizedStorage<T> {
    #[inline]
    pub const fn new() -> Self { Self(PhantomData) }
}

unsafe impl<H: Handle> FromPtr for ZeroSizedStorage<H> {
    #[inline]
    unsafe fn from_ptr(&self, _: NonNull<u8>) -> Self::Handle { H::dangling(MAX_ALIGN) }
}

unsafe impl<H: Handle> SharedGetMut for ZeroSizedStorage<H> {
    #[inline]
    unsafe fn shared_get_mut(&self, _: Self::Handle) -> NonNull<u8> { DANGLING }
}

unsafe impl<H: Handle> Storage for ZeroSizedStorage<H> {
    type Handle = H;

    #[inline]
    unsafe fn get(&self, _: Self::Handle) -> NonNull<u8> { DANGLING }

    #[inline]
    unsafe fn get_mut(&mut self, _: Self::Handle) -> NonNull<u8> { DANGLING }

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
        self.shared_allocate(layout)
    }

    #[inline]
    unsafe fn deallocate(&mut self, _: Self::Handle, _: Layout) {}

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout.into()))
    }

    #[inline]
    fn allocate_zeroed(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.allocate(layout)
    }
}

unsafe impl<H: Handle> ResizableStorage for ZeroSizedStorage<H> {
    #[inline]
    unsafe fn grow(
        &mut self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.allocate(new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.allocate(new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        core::intrinsics::assume(new.size() == 0);
        self.allocate(new)
    }
}

unsafe impl<H: Handle> SharedStorage for ZeroSizedStorage<H> {
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
    fn shared_allocate(&self, layout: Layout) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if layout.size() == 0 && core::intrinsics::likely(layout.align() <= MAX_ALIGN) {
            Ok(MemoryBlock {
                handle: unsafe { H::dangling(MAX_ALIGN) },
                size: 0,
            })
        } else {
            Err(AllocErr::new(layout))
        }
    }

    #[inline]
    unsafe fn shared_deallocate(&self, _: Self::Handle, _: Layout) {}

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        Err(AllocErr::new(layout.into()))
    }

    #[inline]
    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.shared_allocate(layout)
    }
}

unsafe impl<H: Handle> SharedResizableStorage for ZeroSizedStorage<H> {
    #[inline]
    unsafe fn shared_grow(
        &self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.shared_allocate(new)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        self.shared_allocate(new)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        _: Self::Handle,
        _: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        core::intrinsics::assume(new.size() == 0);
        self.shared_allocate(new)
    }
}

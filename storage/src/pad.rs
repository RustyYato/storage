use crate::{
    FromPtr, MultiStorage, NonEmptyLayout, OffsetHandle, ResizableStorage, SharedGetMut, SharedOffsetHandle,
    SharedResizableStorage, SharedStorage, Storage,
};
use core::{alloc::Layout, ptr::NonNull};

#[repr(transparent)]
pub struct Pad<S: ?Sized, const SIZE: usize, const ALIGN: usize> {
    pub storage: S,
}

fn pad<const SIZE: usize, const ALIGN: usize>(layout: Layout) -> Layout {
    assert!(ALIGN.is_power_of_two());
    Layout::from_size_align(layout.size().max(SIZE), layout.align().max(ALIGN))
        .unwrap()
        .pad_to_align()
}

unsafe fn pad_unchecked<const SIZE: usize, const ALIGN: usize>(layout: Layout) -> Layout {
    Layout::from_size_align_unchecked(layout.size().max(SIZE), layout.align().max(ALIGN)).pad_to_align()
}

impl<S: ?Sized, const SIZE: usize, const ALIGN: usize> Pad<S, SIZE, ALIGN> {
    fn pad_ne(layout: NonEmptyLayout) -> NonEmptyLayout {
        unsafe { NonEmptyLayout::new_unchecked(pad::<SIZE, ALIGN>(layout.into())) }
    }

    unsafe fn pad_ne_unchecked(layout: NonEmptyLayout) -> NonEmptyLayout {
        NonEmptyLayout::new_unchecked(pad_unchecked::<SIZE, ALIGN>(layout.into()))
    }

    fn pad(layout: Layout) -> Result<Layout, NonEmptyLayout> {
        let layout = pad::<SIZE, ALIGN>(layout);
        if SIZE == 0 {
            Ok(layout)
        } else {
            Err(unsafe { NonEmptyLayout::new_unchecked(layout) })
        }
    }

    unsafe fn pad_unchecked(layout: Layout) -> Result<Layout, NonEmptyLayout> {
        let layout = pad_unchecked::<SIZE, ALIGN>(layout);
        if SIZE == 0 {
            Ok(layout)
        } else {
            Err(NonEmptyLayout::new_unchecked(layout))
        }
    }

    // pad_nobranch
    fn pad_nb(layout: Layout) -> Layout { pad::<SIZE, ALIGN>(layout) }

    unsafe fn pad_nb_unchecked(layout: Layout) -> Layout { pad_unchecked::<SIZE, ALIGN>(layout) }
}

unsafe impl<S: FromPtr + ?Sized, const SIZE: usize, const ALIGN: usize> FromPtr for Pad<S, SIZE, ALIGN> {
    unsafe fn from_ptr(&self, ptr: NonNull<u8>, layout: Layout) -> Self::Handle {
        S::from_ptr(&self.storage, ptr, layout)
    }
}

unsafe impl<S: OffsetHandle + ?Sized, const SIZE: usize, const ALIGN: usize> OffsetHandle for Pad<S, SIZE, ALIGN> {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle {
        S::offset(&mut self.storage, handle, offset)
    }
}

unsafe impl<S: SharedOffsetHandle + ?Sized, const SIZE: usize, const ALIGN: usize> SharedOffsetHandle
    for Pad<S, SIZE, ALIGN>
{
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle {
        S::shared_offset(&self.storage, handle, offset)
    }
}

impl<S: MultiStorage + ?Sized, const SIZE: usize, const ALIGN: usize> MultiStorage for Pad<S, SIZE, ALIGN> {}
unsafe impl<S: Storage + ?Sized, const SIZE: usize, const ALIGN: usize> Storage for Pad<S, SIZE, ALIGN> {
    type Handle = S::Handle;

    #[inline]
    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { S::get(&self.storage, handle) }

    #[inline]
    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { S::get_mut(&mut self.storage, handle) }

    #[inline]
    fn allocate_nonempty(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        let layout = Self::pad_ne(layout);
        S::allocate_nonempty(&mut self.storage, layout)
    }

    #[inline]
    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        let layout = Self::pad_ne_unchecked(layout);
        S::deallocate_nonempty(&mut self.storage, handle, layout)
    }

    #[inline]
    fn allocate(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match Self::pad(layout) {
            Ok(layout) => S::allocate(&mut self.storage, layout),
            Err(layout) => S::allocate_nonempty(&mut self.storage, layout).map(Into::into),
        }
    }

    #[inline]
    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: core::alloc::Layout) {
        match Self::pad_unchecked(layout) {
            Ok(layout) => S::deallocate(&mut self.storage, handle, layout),
            Err(layout) => S::deallocate_nonempty(&mut self.storage, handle, layout),
        }
    }

    #[inline]
    fn allocate_nonempty_zeroed(
        &mut self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        let layout = Self::pad_ne(layout);
        S::allocate_nonempty_zeroed(&mut self.storage, layout)
    }

    #[inline]
    fn allocate_zeroed(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match Self::pad(layout) {
            Ok(layout) => S::allocate_zeroed(&mut self.storage, layout),
            Err(layout) => S::allocate_nonempty_zeroed(&mut self.storage, layout).map(Into::into),
        }
    }
}

unsafe impl<S: SharedGetMut + ?Sized, const SIZE: usize, const ALIGN: usize> SharedGetMut for Pad<S, SIZE, ALIGN> {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { S::shared_get_mut(&self.storage, handle) }
}

unsafe impl<S: ResizableStorage + ?Sized, const SIZE: usize, const ALIGN: usize> ResizableStorage
    for Pad<S, SIZE, ALIGN>
{
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let new = Self::pad_nb(new);
        let old = Self::pad_nb_unchecked(old);
        S::grow(&mut self.storage, handle, old, new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let new = Self::pad_nb(new);
        let old = Self::pad_nb_unchecked(old);
        S::grow_zeroed(&mut self.storage, handle, old, new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let new = Self::pad_nb(new);
        let old = Self::pad_nb_unchecked(old);
        S::shrink(&mut self.storage, handle, old, new)
    }
}

unsafe impl<S: SharedStorage + ?Sized, const SIZE: usize, const ALIGN: usize> SharedStorage for Pad<S, SIZE, ALIGN> {
    #[inline]
    fn shared_allocate_nonempty(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        let layout = Self::pad_ne(layout);
        S::shared_allocate_nonempty(&self.storage, layout)
    }

    #[inline]
    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: crate::NonEmptyLayout) {
        let layout = Self::pad_ne_unchecked(layout);
        S::shared_deallocate_nonempty(&self.storage, handle, layout)
    }

    #[inline]
    fn shared_allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match Self::pad(layout) {
            Ok(layout) => S::shared_allocate(&self.storage, layout),
            Err(layout) => S::shared_allocate_nonempty(&self.storage, layout).map(Into::into),
        }
    }

    #[inline]
    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: core::alloc::Layout) {
        match Self::pad_unchecked(layout) {
            Ok(layout) => S::shared_deallocate(&self.storage, handle, layout),
            Err(layout) => S::shared_deallocate_nonempty(&self.storage, handle, layout),
        }
    }

    #[inline]
    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: crate::NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, crate::AllocErr> {
        let layout = Self::pad_ne(layout);
        S::shared_allocate_nonempty_zeroed(&self.storage, layout)
    }

    #[inline]
    fn shared_allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        match Self::pad(layout) {
            Ok(layout) => S::shared_allocate_zeroed(&self.storage, layout),
            Err(layout) => S::shared_allocate_nonempty_zeroed(&self.storage, layout).map(Into::into),
        }
    }
}

unsafe impl<S: SharedResizableStorage + ?Sized, const SIZE: usize, const ALIGN: usize> SharedResizableStorage
    for Pad<S, SIZE, ALIGN>
{
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let new = Self::pad_nb(new);
        let old = Self::pad_nb_unchecked(old);
        S::shared_grow(&self.storage, handle, old, new)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let new = Self::pad_nb(new);
        let old = Self::pad_nb_unchecked(old);
        S::shared_grow_zeroed(&self.storage, handle, old, new)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: core::alloc::Layout,
        new: core::alloc::Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, crate::AllocErr> {
        let new = Self::pad_nb(new);
        let old = Self::pad_nb_unchecked(old);
        S::shared_shrink(&self.storage, handle, old, new)
    }
}

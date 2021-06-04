#![allow(clippy::cast_possible_wrap)]

use core::{alloc::Layout, convert::TryFrom, marker::PhantomData, mem, num::NonZeroUsize, ptr::NonNull};

use crate::{
    AllocErr, Handle, MemoryBlock, NonEmptyLayout, NonEmptyMemoryBlock, ResizableStorage, SharedGetMut,
    SharedResizableStorage, SharedStorage, Storage,
};

struct CoVariant<T>(fn() -> T);

pub trait LayoutProvider {
    const SIZE: usize;
    const ALIGN: usize;
}

pub struct TypedLayoutProvider<T>(CoVariant<T>);
pub struct ConstLayoutProvider<const SIZE: usize, const ALIGN: usize>;

impl<T> LayoutProvider for TypedLayoutProvider<T> {
    const SIZE: usize = mem::size_of::<T>();
    const ALIGN: usize = mem::align_of::<T>();
}

impl<const SIZE: usize, const ALIGN: usize> LayoutProvider for ConstLayoutProvider<SIZE, ALIGN> {
    const SIZE: usize = SIZE;
    const ALIGN: usize = ALIGN;
}

#[repr(transparent)]
pub struct AffixStorage<Pre, Suf, S: ?Sized> {
    __: PhantomData<CoVariant<(Pre, Suf)>>,
    pub inner: S,
}

#[repr(transparent)]
pub struct AffixHandle<Pre, Suf, H: ?Sized> {
    __: PhantomData<CoVariant<(Pre, Suf)>>,
    inner: H,
}

pub unsafe fn split<Pre: LayoutProvider, Suf: LayoutProvider>(
    ptr: NonNull<u8>,
    layout: Layout,
) -> (NonNull<u8>, NonNull<u8>) {
    let (_, prefix, suffix) = AffixStorage::<Pre, Suf, ()>::surround_unchecked(layout);

    let ptr = ptr.as_ptr();
    (
        NonNull::new_unchecked(ptr.sub(prefix).cast()),
        NonNull::new_unchecked(ptr.add(suffix - prefix).cast()),
    )
}

impl<Pre, Suf, S> AffixStorage<Pre, Suf, S> {
    #[inline]
    pub const fn new(storage: S) -> Self {
        Self {
            inner: storage,
            __: PhantomData,
        }
    }
}

impl<Pre: LayoutProvider, Suf: LayoutProvider, S> AffixStorage<Pre, Suf, S> {
    const NO_AFFIX: bool = Pre::SIZE == 0 && Pre::ALIGN == 1 && Suf::SIZE == 0 && Suf::ALIGN == 1;

    #[inline]
    fn surround(layout: Layout) -> Option<(Layout, usize, usize)> {
        let (layout, offset) = Layout::from_size_align(Pre::SIZE, Pre::ALIGN)
            .unwrap()
            .extend(layout)
            .ok()?;
        let (layout, suffix) = layout
            .extend(Layout::from_size_align(Suf::SIZE, Suf::ALIGN).unwrap())
            .ok()?;
        debug_assert!(isize::try_from(offset).is_ok());
        Some((layout, offset, suffix))
    }

    unsafe fn surround_unchecked(layout: Layout) -> (Layout, usize, usize) {
        match Self::surround(layout) {
            Some(x) => x,
            None => core::hint::unreachable_unchecked(),
        }
    }

    /// # Safety
    ///
    /// `ptr` must be aquired from `Self::*get*`
    /// `ptr` must have been allocated with `layout`
    #[allow(clippy::unused_self)]
    pub unsafe fn split_untyped(&self, ptr: NonNull<u8>, layout: Layout) -> (NonNull<u8>, NonNull<u8>) {
        split::<Pre, Suf>(ptr, layout)
    }
}

impl<Pre, Suf, S> AffixStorage<TypedLayoutProvider<Pre>, TypedLayoutProvider<Suf>, S> {
    /// # Safety
    ///
    /// `ptr` must be aquired from `Self::*get*`
    /// `ptr` must have been allocated with `layout`
    #[allow(clippy::unused_self)]
    pub unsafe fn split(&self, ptr: NonNull<u8>, layout: Layout) -> (NonNull<Pre>, NonNull<Suf>) {
        let (pre, suf) = self.split_untyped(ptr, layout);
        (pre.cast(), suf.cast())
    }
}

impl<Pre: LayoutProvider, Suf: LayoutProvider, S: Copy> Copy for AffixStorage<Pre, Suf, S> {}
impl<Pre: LayoutProvider, Suf: LayoutProvider, S: Clone> Clone for AffixStorage<Pre, Suf, S> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            __: PhantomData,
            inner: self.inner.clone(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) { self.inner.clone_from(&source.inner) }
}

impl<Pre: LayoutProvider, Suf: LayoutProvider, H: Copy> Copy for AffixHandle<Pre, Suf, H> {}
impl<Pre: LayoutProvider, Suf: LayoutProvider, H: Clone> Clone for AffixHandle<Pre, Suf, H> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            __: PhantomData,
            inner: self.inner.clone(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) { self.inner.clone_from(&source.inner) }
}

unsafe impl<Pre: LayoutProvider, Suf: LayoutProvider, H: Handle> Handle for AffixHandle<Pre, Suf, H> {
    unsafe fn dangling(align: usize) -> Self {
        Self {
            __: PhantomData,
            inner: H::dangling(align),
        }
    }
}

pub unsafe trait OffsetHandle: Storage {
    unsafe fn offset(&mut self, handle: Self::Handle, offset: isize) -> Self::Handle;
}

pub unsafe trait SharedOffsetHandle: OffsetHandle + SharedStorage {
    unsafe fn shared_offset(&self, handle: Self::Handle, offset: isize) -> Self::Handle;
}

unsafe impl<Pre: LayoutProvider, Suf: LayoutProvider, S: SharedGetMut + OffsetHandle> SharedGetMut
    for AffixStorage<Pre, Suf, S>
{
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> NonNull<u8> { self.inner.shared_get_mut(handle.inner) }
}

unsafe impl<Pre: LayoutProvider, Suf: LayoutProvider, S: OffsetHandle> Storage for AffixStorage<Pre, Suf, S> {
    type Handle = AffixHandle<Pre, Suf, S::Handle>;

    unsafe fn get(&self, handle: Self::Handle) -> NonNull<u8> { self.inner.get(handle.inner) }

    unsafe fn get_mut(&mut self, handle: Self::Handle) -> NonNull<u8> { self.inner.get_mut(handle.inner) }

    fn allocate_nonempty(&mut self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let (layout, prefix, _suffix) = Self::surround(layout.into()).ok_or_else(|| AllocErr::new(layout.into()))?;

        let memory_block = self
            .inner
            .allocate_nonempty(unsafe { NonEmptyLayout::new_unchecked(layout) })?;

        Ok(NonEmptyMemoryBlock {
            size: unsafe { NonZeroUsize::new_unchecked(layout.size()) },
            handle: AffixHandle {
                __: PhantomData,
                inner: unsafe { self.inner.offset(memory_block.handle, prefix as isize) },
            },
        })
    }

    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout) {
        let (layout, prefix, _suffix) = Self::surround_unchecked(layout.into());
        let prefix = prefix as isize;
        let handle = self.inner.offset(handle.inner, -prefix);
        self.inner
            .deallocate_nonempty(handle, NonEmptyLayout::new_unchecked(layout))
    }

    fn allocate(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        let (layout, prefix, _suffix) = Self::surround(layout).ok_or_else(|| AllocErr::new(layout))?;

        let memory_block = if Self::NO_AFFIX {
            self.inner.allocate(layout)
        } else {
            self.inner
                .allocate_nonempty(unsafe { NonEmptyLayout::new_unchecked(layout) })
                .map(Into::into)
        };
        let memory_block = memory_block?;

        Ok(MemoryBlock {
            size: layout.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: unsafe { self.inner.offset(memory_block.handle, prefix as isize) },
            },
        })
    }

    unsafe fn deallocate(&mut self, handle: Self::Handle, layout: Layout) {
        let (layout, prefix, _suffix) = Self::surround_unchecked(layout);
        let prefix = prefix as isize;
        let handle = self.inner.offset(handle.inner, -prefix);
        if Self::NO_AFFIX {
            self.inner.deallocate(handle, layout)
        } else {
            self.inner
                .deallocate_nonempty(handle, NonEmptyLayout::new_unchecked(layout))
        }
    }

    fn allocate_nonempty_zeroed(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let (layout, prefix, _suffix) = Self::surround(layout.into()).ok_or_else(|| AllocErr::new(layout.into()))?;

        let memory_block = self
            .inner
            .allocate_nonempty_zeroed(unsafe { NonEmptyLayout::new_unchecked(layout) })?;

        Ok(NonEmptyMemoryBlock {
            size: unsafe { NonZeroUsize::new_unchecked(layout.size()) },
            handle: AffixHandle {
                __: PhantomData,
                inner: unsafe { self.inner.offset(memory_block.handle, prefix as isize) },
            },
        })
    }

    fn allocate_zeroed(&mut self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        let (layout, prefix, _suffix) = Self::surround(layout).ok_or_else(|| AllocErr::new(layout))?;

        let memory_block = if Self::NO_AFFIX {
            self.inner.allocate_zeroed(layout)
        } else {
            self.inner
                .allocate_nonempty_zeroed(unsafe { NonEmptyLayout::new_unchecked(layout) })
                .map(Into::into)
        };
        let memory_block = memory_block?;

        Ok(MemoryBlock {
            size: layout.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: unsafe { self.inner.offset(memory_block.handle, prefix as isize) },
            },
        })
    }
}

unsafe impl<Pre: LayoutProvider, Suf: LayoutProvider, S: ResizableStorage + OffsetHandle> ResizableStorage
    for AffixStorage<Pre, Suf, S>
{
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if Self::NO_AFFIX {
            return self.inner.grow(handle.inner, old, new).map(|memory_block| MemoryBlock {
                size: memory_block.size,
                handle: AffixHandle {
                    __: PhantomData,
                    inner: memory_block.handle,
                },
            })
        }

        let (new, new_pre, new_suf) = Self::surround(new).ok_or_else(|| AllocErr::new(new))?;
        let (old, _old_pre, old_suf) = Self::surround_unchecked(old);

        let memory_block = self.inner.grow(handle.inner, old, new)?;

        if Suf::SIZE != 0 {
            let ptr = self.inner.get_mut(memory_block.handle).as_ptr();
            ptr.add(old_suf).copy_to(ptr.add(new_suf), Suf::SIZE)
        }

        Ok(MemoryBlock {
            size: new.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: self.inner.offset(memory_block.handle, new_pre as isize),
            },
        })
    }

    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if Self::NO_AFFIX {
            return self
                .inner
                .grow_zeroed(handle.inner, old, new)
                .map(|memory_block| MemoryBlock {
                    size: memory_block.size,
                    handle: AffixHandle {
                        __: PhantomData,
                        inner: memory_block.handle,
                    },
                })
        }

        let (new, new_pre, new_suf) = Self::surround(new).ok_or_else(|| AllocErr::new(new))?;
        let (old, _old_pre, old_suf) = Self::surround_unchecked(old);

        let memory_block = self.inner.grow_zeroed(handle.inner, old, new)?;

        if Suf::SIZE != 0 {
            let ptr = self.inner.get_mut(memory_block.handle).as_ptr();
            ptr.add(old_suf).copy_to(ptr.add(new_suf), Suf::SIZE);
            let zero_count = Suf::SIZE.min(new_suf - old_suf);
            ptr.add(old_suf).write_bytes(0, zero_count);
        }

        Ok(MemoryBlock {
            size: new.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: self.inner.offset(memory_block.handle, new_pre as isize),
            },
        })
    }

    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if Self::NO_AFFIX {
            return self
                .inner
                .shrink(handle.inner, old, new)
                .map(|memory_block| MemoryBlock {
                    size: memory_block.size,
                    handle: AffixHandle {
                        __: PhantomData,
                        inner: memory_block.handle,
                    },
                })
        }

        let (old, _old_pre, old_suf) = Self::surround_unchecked(old);
        let (new, new_pre, new_suf) = Self::surround_unchecked(new);

        if Suf::SIZE != 0 {
            let ptr = self.inner.get_mut(handle.inner).as_ptr();
            ptr.add(old_suf).copy_to(ptr.add(new_suf), Suf::SIZE);
        }

        let memory_block = self.inner.shrink(handle.inner, old, new)?;

        Ok(MemoryBlock {
            size: new.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: self.inner.offset(memory_block.handle, new_pre as isize),
            },
        })
    }
}

unsafe impl<Pre: LayoutProvider, Suf: LayoutProvider, S: SharedOffsetHandle> SharedStorage
    for AffixStorage<Pre, Suf, S>
{
    fn shared_allocate_nonempty(&self, layout: NonEmptyLayout) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let (layout, prefix, _suffix) = Self::surround(layout.into()).ok_or_else(|| AllocErr::new(layout.into()))?;

        let memory_block = self
            .inner
            .shared_allocate_nonempty(unsafe { NonEmptyLayout::new_unchecked(layout) })?;

        Ok(NonEmptyMemoryBlock {
            size: unsafe { NonZeroUsize::new_unchecked(layout.size()) },
            handle: AffixHandle {
                __: PhantomData,
                inner: unsafe { self.inner.shared_offset(memory_block.handle, prefix as isize) },
            },
        })
    }

    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout) {
        let (layout, prefix, _suffix) = Self::surround_unchecked(layout.into());
        let prefix = prefix as isize;
        let handle = self.inner.shared_offset(handle.inner, -prefix);
        self.inner
            .shared_deallocate_nonempty(handle, NonEmptyLayout::new_unchecked(layout))
    }

    fn shared_allocate(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        let (layout, prefix, _suffix) = Self::surround(layout).ok_or_else(|| AllocErr::new(layout))?;

        let memory_block = if Self::NO_AFFIX {
            self.inner.shared_allocate(layout)
        } else {
            self.inner
                .shared_allocate_nonempty(unsafe { NonEmptyLayout::new_unchecked(layout) })
                .map(Into::into)
        };
        let memory_block = memory_block?;

        Ok(MemoryBlock {
            size: layout.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: unsafe { self.inner.shared_offset(memory_block.handle, prefix as isize) },
            },
        })
    }

    unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: Layout) {
        let (layout, prefix, _suffix) = Self::surround_unchecked(layout);
        let prefix = prefix as isize;
        let handle = self.inner.shared_offset(handle.inner, -prefix);
        if Self::NO_AFFIX {
            self.inner.shared_deallocate(handle, layout)
        } else {
            self.inner
                .shared_deallocate_nonempty(handle, NonEmptyLayout::new_unchecked(layout))
        }
    }

    fn shared_allocate_nonempty_zeroed(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let (layout, prefix, _suffix) = Self::surround(layout.into()).ok_or_else(|| AllocErr::new(layout.into()))?;

        let memory_block = self
            .inner
            .shared_allocate_nonempty_zeroed(unsafe { NonEmptyLayout::new_unchecked(layout) })?;

        Ok(NonEmptyMemoryBlock {
            size: unsafe { NonZeroUsize::new_unchecked(layout.size()) },
            handle: AffixHandle {
                __: PhantomData,
                inner: unsafe { self.inner.shared_offset(memory_block.handle, prefix as isize) },
            },
        })
    }

    fn shared_allocate_zeroed(&self, layout: Layout) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        let (layout, prefix, _suffix) = Self::surround(layout).ok_or_else(|| AllocErr::new(layout))?;

        let memory_block = if Self::NO_AFFIX {
            self.inner.shared_allocate_zeroed(layout)
        } else {
            self.inner
                .shared_allocate_nonempty_zeroed(unsafe { NonEmptyLayout::new_unchecked(layout) })
                .map(Into::into)
        };
        let memory_block = memory_block?;

        Ok(MemoryBlock {
            size: layout.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: unsafe { self.inner.shared_offset(memory_block.handle, prefix as isize) },
            },
        })
    }
}

unsafe impl<Pre: LayoutProvider, Suf: LayoutProvider, S: SharedResizableStorage + SharedOffsetHandle>
    SharedResizableStorage for AffixStorage<Pre, Suf, S>
{
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if Self::NO_AFFIX {
            return self
                .inner
                .shared_grow(handle.inner, old, new)
                .map(|memory_block| MemoryBlock {
                    size: memory_block.size,
                    handle: AffixHandle {
                        __: PhantomData,
                        inner: memory_block.handle,
                    },
                })
        }

        let (new, new_pre, new_suf) = Self::surround(new).ok_or_else(|| AllocErr::new(new))?;
        let (old, _old_pre, old_suf) = Self::surround_unchecked(old);

        let memory_block = self.inner.shared_grow(handle.inner, old, new)?;

        if Suf::SIZE != 0 {
            let ptr = self.inner.shared_get_mut(memory_block.handle).as_ptr();
            ptr.add(old_suf).copy_to(ptr.add(new_suf), Suf::SIZE)
        }

        Ok(MemoryBlock {
            size: new.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: self.inner.shared_offset(memory_block.handle, new_pre as isize),
            },
        })
    }

    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if Self::NO_AFFIX {
            return self
                .inner
                .shared_grow_zeroed(handle.inner, old, new)
                .map(|memory_block| MemoryBlock {
                    size: memory_block.size,
                    handle: AffixHandle {
                        __: PhantomData,
                        inner: memory_block.handle,
                    },
                })
        }

        let (new, new_pre, new_suf) = Self::surround(new).ok_or_else(|| AllocErr::new(new))?;
        let (old, _old_pre, old_suf) = Self::surround_unchecked(old);

        let memory_block = self.inner.shared_grow_zeroed(handle.inner, old, new)?;

        if Suf::SIZE != 0 {
            let ptr = self.inner.shared_get_mut(memory_block.handle).as_ptr();
            ptr.add(old_suf).copy_to(ptr.add(new_suf), Suf::SIZE);
            let zero_count = Suf::SIZE.min(new_suf - old_suf);
            ptr.add(old_suf).write_bytes(0, zero_count);
        }

        Ok(MemoryBlock {
            size: new.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: self.inner.shared_offset(memory_block.handle, new_pre as isize),
            },
        })
    }

    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<MemoryBlock<Self::Handle>, AllocErr> {
        if Self::NO_AFFIX {
            return self
                .inner
                .shared_shrink(handle.inner, old, new)
                .map(|memory_block| MemoryBlock {
                    size: memory_block.size,
                    handle: AffixHandle {
                        __: PhantomData,
                        inner: memory_block.handle,
                    },
                })
        }

        let (old, _old_pre, old_suf) = Self::surround_unchecked(old);
        let (new, new_pre, new_suf) = Self::surround_unchecked(new);

        if Suf::SIZE != 0 {
            let ptr = self.inner.shared_get_mut(handle.inner).as_ptr();
            ptr.add(old_suf).copy_to(ptr.add(new_suf), Suf::SIZE);
        }

        let memory_block = self.inner.shared_shrink(handle.inner, old, new)?;

        Ok(MemoryBlock {
            size: new.size(),
            handle: AffixHandle {
                __: PhantomData,
                inner: self.inner.shared_offset(memory_block.handle, new_pre as isize),
            },
        })
    }
}

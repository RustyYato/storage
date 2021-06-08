use core::{
    alloc::{Layout, LayoutError},
    cell::Cell,
    mem::MaybeUninit,
    num::NonZeroUsize,
    slice,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{
    AllocErr, FromPtr, Handle, NonEmptyLayout, NonEmptyMemoryBlock, ResizableStorage, SharedGetMut,
    SharedResizableStorage, SharedStorage, Storage,
};

pub trait Flush {
    fn try_flush(&mut self) -> bool;

    fn flush(&mut self) { while !self.try_flush() {} }
}

pub trait SharedFlush: Flush {
    fn try_shared_flush(&self) -> bool;

    fn shared_flush(&self) { while !self.try_shared_flush() {} }
}

struct FreeListItem<H> {
    layout: Cell<Layout>,
    handle: Cell<H>,
}

pub struct FreeListStorage<S: Storage> {
    max_length: NonZeroUsize,
    storage: S,
    items: S::Handle,
}

impl<S: Storage> Drop for FreeListStorage<S> {
    fn drop(&mut self) {
        unsafe {
            let (layout, ..) = unwrap_unchecked(free_list_layout::<S::Handle>(self.max_length.get()));
            self.storage
                .deallocate_nonempty(self.items, NonEmptyLayout::new_unchecked(layout));
        }
    }
}

const MASK_STATUS: u8 = !SINGLE_LOCK;

const SINGLE_LOCK: u8 = 0b1000_0000;
const SINGLE_STATUS: u8 = 1;

fn free_list_layout<H>(max_size: usize) -> Result<(Layout, usize, usize), LayoutError> {
    let bitflags_len = (max_size / 7) + usize::from(max_size % 7 != 0);
    let fl = Layout::new::<FreeListItem<H>>().repeat(max_size)?.0;
    let bf = Layout::new::<AtomicU8>().repeat(bitflags_len)?.0;
    fl.extend(bf).map(|(layout, bitflags)| (layout, bitflags, bitflags_len))
}

#[allow(clippy::missing_const_for_fn)]
unsafe fn unwrap_unchecked<T, E>(result: Result<T, E>) -> T {
    match result {
        Ok(x) => x,
        Err(_) => core::hint::unreachable_unchecked(),
    }
}

impl<S: Storage> FreeListStorage<S> {
    pub fn new(max_size: NonZeroUsize, storage: S) -> Self {
        Self::try_new(max_size, storage).unwrap_or_else(AllocErr::handle)
    }

    /// # Panics
    ///
    /// * If layout could not be computed TODO
    pub fn try_new(max_size: NonZeroUsize, mut storage: S) -> Result<Self, AllocErr<S>> {
        let (layout, freelist, freelist_len) = free_list_layout::<S::Handle>(max_size.get()).unwrap();
        let layout = unsafe { NonEmptyLayout::new_unchecked(layout) };
        let meta = match storage.allocate_nonempty(layout) {
            Ok(x) => x.handle,
            Err(err) => return Err(err.with(storage)),
        };
        let items_ptr = unsafe { storage.get_mut(meta) };
        let ptr = items_ptr.cast::<MaybeUninit<FreeListItem<S::Handle>>>().as_ptr();
        let items = unsafe { slice::from_raw_parts_mut(ptr, max_size.get()) };

        let dangling = unsafe { Handle::dangling(1) };
        for free in items {
            *free = MaybeUninit::new(FreeListItem {
                layout: Cell::new(Layout::new::<()>()),
                handle: Cell::new(dangling),
            });
        }

        let bitflags = unsafe {
            slice::from_raw_parts_mut(items_ptr.as_ptr().cast::<MaybeUninit<u8>>().add(freelist), freelist_len)
        };
        bitflags.fill(MaybeUninit::new(0));

        Ok(Self {
            max_length: max_size,
            storage,
            items: meta,
        })
    }
}

impl<S: Storage> FreeListStorage<S> {
    fn free_list(&self) -> (&[FreeListItem<S::Handle>], &[AtomicU8]) {
        let (_, bitflags, bitflags_len) =
            unsafe { unwrap_unchecked(free_list_layout::<S::Handle>(self.max_length.get())) };
        let meta_array = unsafe { self.storage.get(self.items) };
        let free_list = meta_array.cast::<FreeListItem<S::Handle>>().as_ptr();
        unsafe {
            let bitflags = free_list.cast::<AtomicU8>().add(bitflags);
            (
                slice::from_raw_parts(free_list, self.max_length.get()),
                slice::from_raw_parts(bitflags, bitflags_len),
            )
        }
    }

    fn free_list_mut(&mut self) -> (&mut [FreeListItem<S::Handle>], &mut [u8]) {
        let (_, bitflags, bitflags_len) =
            unsafe { unwrap_unchecked(free_list_layout::<S::Handle>(self.max_length.get())) };
        unsafe { self.free_list_mut_at(bitflags, bitflags_len) }
    }

    unsafe fn free_list_at(&self, bitflags: usize, bitflags_len: usize) -> (&[FreeListItem<S::Handle>], &[AtomicU8]) {
        let meta_array = self.storage.get(self.items);
        let free_list = meta_array.cast::<FreeListItem<S::Handle>>().as_ptr();
        let bitflags = free_list.cast::<AtomicU8>().add(bitflags);
        (
            slice::from_raw_parts(free_list, self.max_length.get()),
            slice::from_raw_parts(bitflags, bitflags_len),
        )
    }

    unsafe fn free_list_mut_at(
        &mut self,
        bitflags: usize,
        bitflags_len: usize,
    ) -> (&mut [FreeListItem<S::Handle>], &mut [u8]) {
        let meta_array = self.storage.get_mut(self.items);
        let free_list = meta_array.cast::<FreeListItem<S::Handle>>().as_ptr();
        let bitflags = free_list.cast::<u8>().add(bitflags);
        (
            slice::from_raw_parts_mut(free_list, self.max_length.get()),
            slice::from_raw_parts_mut(bitflags, bitflags_len),
        )
    }

    fn attempt_allocate(
        free_list: &mut [FreeListItem<S::Handle>],
        bitflags: &mut [u8],
        layout: NonEmptyLayout,
    ) -> Option<NonEmptyMemoryBlock<S::Handle>> {
        for (i, owned) in bitflags.iter_mut().enumerate() {
            // if all of the slots are empty, skip this bucket
            // NOTE: because we have `&mut self`, the free list can't be locked
            if *owned == 0 {
                continue
            }

            for j in 0..7 {
                let status_bit = SINGLE_STATUS << j;
                if (*owned & status_bit) != 0 {
                    let index = i * 7 + j;
                    let free_list = unsafe { free_list.get_unchecked_mut(index) };
                    let item_layout = free_list.layout.get();

                    if item_layout.align() == layout.align() && item_layout.size() >= layout.size() {
                        *owned &= !status_bit;

                        return Some(NonEmptyMemoryBlock {
                            handle: free_list.handle.get(),
                            size: unsafe { NonZeroUsize::new_unchecked(layout.size()) },
                        })
                    }
                }
            }
        }

        None
    }

    fn attempt_deallocate(
        free_list: &mut [FreeListItem<S::Handle>],
        bitflags: &mut [u8],
        handle: S::Handle,
        layout: NonEmptyLayout,
    ) -> bool {
        for (i, owned) in bitflags.iter_mut().enumerate() {
            // if all of the slots are full, skip this bucket
            // NOTE: because we have `&mut self`, the free list can't be locked
            if *owned == MASK_STATUS {
                continue
            }

            for j in 0..7 {
                let status_bit = SINGLE_STATUS << j;
                if (*owned & status_bit) == 0 {
                    *owned |= status_bit;
                    let index = i * 7 + j;
                    let free_list = unsafe { free_list.get_unchecked_mut(index) };
                    free_list.layout = Cell::new(layout.into());
                    free_list.handle = Cell::new(handle);
                    return true
                }
            }
        }

        false
    }
}

impl<S: SharedStorage> FreeListStorage<S> {
    fn attempt_shared_allocate(
        free_list: &[FreeListItem<S::Handle>],
        bitflags: &[AtomicU8],
        layout: NonEmptyLayout,
        was_blocked: &mut bool,
    ) -> Option<NonEmptyMemoryBlock<S::Handle>> {
        for (i, owned) in bitflags.iter().enumerate() {
            let fetch = owned.load(Ordering::Relaxed);

            // if the bucket is locked or all of the slots are empty, skip this bucket
            if (fetch & SINGLE_LOCK) != 0 || fetch == 0 {
                *was_blocked |= (fetch & SINGLE_LOCK) != 0;
                continue
            }

            // try to aquire the lock
            let locked = owned.fetch_or(SINGLE_LOCK, Ordering::Acquire);

            // if someone else locked the bucket
            if locked & SINGLE_LOCK != 0 {
                *was_blocked = false;
                continue
            }

            let status = locked;

            for j in 0..7 {
                let status_bit = SINGLE_STATUS << j;
                if (status & status_bit) != 0 {
                    let index = i * 7 + j;
                    let free_list = unsafe { free_list.get_unchecked(index) };
                    let item_layout = free_list.layout.get();

                    if item_layout.align() == layout.align() && item_layout.size() >= layout.size() {
                        let handle = free_list.handle.get();
                        // clear lock and mark this slot as empty
                        owned.store(status & !status_bit, Ordering::Release);

                        return Some(NonEmptyMemoryBlock {
                            handle,
                            size: unsafe { NonZeroUsize::new_unchecked(layout.size()) },
                        })
                    }
                }
            }

            // clear lock
            owned.store(status, Ordering::Release);
        }

        None
    }

    fn attempt_shared_deallocate(
        free_list: &[FreeListItem<S::Handle>],
        bitflags: &[AtomicU8],
        handle: S::Handle,
        layout: NonEmptyLayout,
        was_blocked: &mut bool,
    ) -> bool {
        for (i, owned) in bitflags.iter().enumerate() {
            let fetch = owned.load(Ordering::Relaxed);

            // if the bucket is locked or all of the slots are full, skip this bucket
            if (fetch & SINGLE_LOCK) != 0 || fetch == MASK_STATUS {
                *was_blocked |= (fetch & SINGLE_LOCK) != 0;
                continue
            }

            // try to aquire the lock
            let locked = owned.fetch_or(SINGLE_LOCK, Ordering::Acquire);

            // if someone else locked the bucket
            if locked & SINGLE_LOCK != 0 {
                *was_blocked = false;
                continue
            }

            let status = locked;

            for j in 0..7 {
                let status_bit = SINGLE_STATUS << j;
                if (status & status_bit) == 0 {
                    let index = i * 7 + j;
                    let free_list = unsafe { free_list.get_unchecked(index) };
                    free_list.layout.set(layout.into());
                    free_list.handle.set(handle);

                    // clear lock and mark this slot as full
                    owned.store(status | status_bit, Ordering::Release);
                    return true
                }
            }

            // clear lock
            owned.store(status, Ordering::Release);
        }

        false
    }
}

unsafe impl<S: FromPtr> FromPtr for FreeListStorage<S> {
    unsafe fn from_ptr(&self, ptr: core::ptr::NonNull<u8>, layout: Layout) -> Self::Handle {
        self.storage.from_ptr(ptr, layout)
    }
}

unsafe impl<S: SharedGetMut> SharedGetMut for FreeListStorage<S> {
    unsafe fn shared_get_mut(&self, handle: Self::Handle) -> core::ptr::NonNull<u8> {
        self.storage.shared_get_mut(handle)
    }
}

unsafe impl<S: Storage> Storage for FreeListStorage<S> {
    type Handle = S::Handle;

    unsafe fn get(&self, handle: Self::Handle) -> core::ptr::NonNull<u8> { self.storage.get(handle) }

    unsafe fn get_mut(&mut self, handle: Self::Handle) -> core::ptr::NonNull<u8> { self.storage.get_mut(handle) }

    fn allocate_nonempty(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let (free_list, bitflags) = self.free_list_mut();
        #[allow(clippy::single_match_else)]
        match Self::attempt_allocate(free_list, bitflags, layout) {
            Some(memory_block) => Ok(memory_block),
            None => {
                let memory = self.storage.allocate_nonempty(layout)?;
                Ok(NonEmptyMemoryBlock {
                    handle: memory.handle,
                    size: memory.size,
                })
            }
        }
    }

    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout) {
        let (free_list, bitflags) = self.free_list_mut();
        if !Self::attempt_deallocate(free_list, bitflags, handle, layout) {
            self.storage.deallocate_nonempty(handle, layout)
        }
    }
}

unsafe impl<S: SharedStorage> SharedStorage for FreeListStorage<S> {
    fn shared_allocate_nonempty(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let (free_list, bitflags) = self.free_list();

        let waiter = crate::backoff::Backoff::new();
        while waiter.spin() {
            let mut was_blocked = false;
            if let Some(memory_block) = Self::attempt_shared_allocate(free_list, bitflags, layout, &mut was_blocked) {
                return Ok(memory_block)
            }
            if !was_blocked {
                break
            }
        }

        let memory = self.storage.shared_allocate_nonempty(layout)?;
        Ok(NonEmptyMemoryBlock {
            handle: memory.handle,
            size: memory.size,
        })
    }

    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout) {
        let (free_list, bitflags) = self.free_list();

        let waiter = crate::backoff::Backoff::new();
        while waiter.spin() {
            let mut was_blocked = false;
            if Self::attempt_shared_deallocate(free_list, bitflags, handle, layout, &mut was_blocked) {
                return
            }
            if !was_blocked {
                break
            }
        }

        self.storage.shared_deallocate_nonempty(handle, layout)
    }
}

impl<S: Storage + Flush> FreeListStorage<S> {
    fn shallow_flush(&mut self) {
        type ScratchSpace<H> = crate::SingleStackStorage<[(H, Layout); 7]>;

        let (_, bitflags, bitflags_len) =
            unsafe { unwrap_unchecked(free_list_layout::<S::Handle>(self.max_length.get())) };

        for i in 0..bitflags_len {
            let (freelist, bitflags) = unsafe { self.free_list_mut_at(bitflags, bitflags_len) };

            let flags = unsafe { bitflags.get_unchecked_mut(i) };

            // if the chunk is empty, then skip it
            if *flags == 0 {
                continue
            }

            let mut vec = crate::vec::Vec::new_in(ScratchSpace::<S::Handle>::new());

            let flags = core::mem::take(flags);
            let index = i * 7;
            for j in 0..7 {
                let flag = flags & (1 << j);

                if flag != 0 {
                    let index = index + j;
                    let freelist = unsafe { freelist.get_unchecked_mut(index) };

                    unsafe {
                        vec.push_unchecked((freelist.handle.get(), freelist.layout.get()));
                    }
                }
            }

            while let Some((handle, layout)) = vec.try_pop() {
                unsafe {
                    self.storage
                        .deallocate_nonempty(handle, NonEmptyLayout::new_unchecked(layout))
                }
            }
        }
    }

    fn shared_shallow_flush(&self, force_retry: bool) -> bool
    where
        S: SharedStorage,
    {
        type ScratchSpace<H> = crate::SingleStackStorage<[(H, Layout); 7]>;

        let mut completed = true;

        let (_, bitflags, bitflags_len) =
            unsafe { unwrap_unchecked(free_list_layout::<S::Handle>(self.max_length.get())) };

        let (freelist, bitflags) = unsafe { self.free_list_at(bitflags, bitflags_len) };
        'main_loop: for (i, flags) in bitflags.iter().enumerate() {
            let mut current_flags = flags.load(Ordering::Relaxed);

            loop {
                // if the chunk is empty, then skip it (even if it's locked)
                if (current_flags & !SINGLE_LOCK) == 0 {
                    continue 'main_loop
                }

                // if the chunk is locked, then retry or skip the block
                if (current_flags & SINGLE_LOCK) != 0 {
                    if force_retry {
                        core::hint::spin_loop();
                        current_flags = flags.load(Ordering::Relaxed);
                    } else {
                        completed = false;
                        continue 'main_loop
                    }
                }

                // if the chunk is empty, then skip it
                if let Err(cf) =
                    flags.compare_exchange(current_flags, SINGLE_LOCK, Ordering::Acquire, Ordering::Relaxed)
                {
                    core::hint::spin_loop();
                    current_flags = cf;
                } else {
                    break
                }
            }

            let mut vec = crate::vec::Vec::new_in(ScratchSpace::<S::Handle>::new());

            let index = i * 7;
            for j in 0..7 {
                let flag = current_flags & (1 << j);

                if flag != 0 {
                    let index = index + j;
                    let freelist = unsafe { freelist.get_unchecked(index) };

                    unsafe {
                        vec.push_unchecked((freelist.handle.get(), freelist.layout.get()));
                    }
                }
            }

            flags.store(0, Ordering::Release);

            while let Some((handle, layout)) = vec.try_pop() {
                unsafe {
                    self.storage
                        .shared_deallocate_nonempty(handle, NonEmptyLayout::new_unchecked(layout))
                }
            }
        }

        completed
    }
}

impl<S: Storage + Flush> Flush for FreeListStorage<S> {
    fn try_flush(&mut self) -> bool {
        self.shallow_flush();
        self.storage.try_flush()
    }

    fn flush(&mut self) {
        self.shallow_flush();
        self.storage.flush();
    }
}

impl<S: SharedStorage + SharedFlush> SharedFlush for FreeListStorage<S> {
    fn try_shared_flush(&self) -> bool {
        let shallow = self.shared_shallow_flush(false);
        let storage = self.storage.try_shared_flush();
        shallow && storage
    }

    fn shared_flush(&self) {
        self.shared_shallow_flush(true);
        self.storage.shared_flush();
    }
}

unsafe impl<S: ResizableStorage> ResizableStorage for FreeListStorage<S> {
    #[inline]
    unsafe fn grow(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.grow(handle, old, new)
    }

    #[inline]
    unsafe fn grow_zeroed(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.grow_zeroed(handle, old, new)
    }

    #[inline]
    unsafe fn shrink(
        &mut self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shrink(handle, old, new)
    }
}

unsafe impl<S: SharedResizableStorage> SharedResizableStorage for FreeListStorage<S> {
    #[inline]
    unsafe fn shared_grow(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_grow(handle, old, new)
    }

    #[inline]
    unsafe fn shared_grow_zeroed(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_grow_zeroed(handle, old, new)
    }

    #[inline]
    unsafe fn shared_shrink(
        &self,
        handle: Self::Handle,
        old: Layout,
        new: Layout,
    ) -> Result<crate::MemoryBlock<Self::Handle>, AllocErr> {
        self.storage.shared_shrink(handle, old, new)
    }
}

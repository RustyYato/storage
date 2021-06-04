use core::{
    alloc::{Layout, LayoutError},
    cell::Cell,
    mem::MaybeUninit,
    num::NonZeroUsize,
    slice,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{AllocErr, Handle, NonEmptyLayout, NonEmptyMemoryBlock, SharedGetMut, SharedStorage, Storage};

struct FreeListItem<H> {
    layout: Cell<Layout>,
    handle: Cell<FreeListHandle<H>>,
}

pub struct FreeListStorage<S: Storage> {
    max_size: NonZeroUsize,
    storage: S,
    items: S::Handle,
}

#[derive(Clone, Copy)]
pub struct FreeListHandle<H>(H);

const MASK_STATUS: u8 = !SINGLE_LOCK;

const SINGLE_LOCK: u8 = 0b1000_0000;
const SINGLE_STATUS: u8 = 1;

unsafe impl<H: Handle> Handle for FreeListHandle<H> {
    unsafe fn dangling(align: usize) -> Self { Self(Handle::dangling(align)) }
}

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
            max_size,
            storage,
            items: meta,
        })
    }
}

impl<S: Storage> FreeListStorage<S> {
    fn free_list(&self) -> (&[FreeListItem<S::Handle>], &[AtomicU8]) {
        let (_, bitflags, bitflags_len) =
            unsafe { unwrap_unchecked(free_list_layout::<S::Handle>(self.max_size.get())) };
        let meta_array = unsafe { self.storage.get(self.items) };
        let free_list = meta_array.cast::<FreeListItem<S::Handle>>().as_ptr();
        unsafe {
            let bitflags = free_list.cast::<AtomicU8>().add(bitflags);
            (
                slice::from_raw_parts(free_list, self.max_size.get()),
                slice::from_raw_parts(bitflags, bitflags_len),
            )
        }
    }

    fn free_list_mut(&mut self) -> (&mut [FreeListItem<S::Handle>], &mut [u8]) {
        let (_, bitflags, bitflags_len) =
            unsafe { unwrap_unchecked(free_list_layout::<S::Handle>(self.max_size.get())) };
        let meta_array = unsafe { self.storage.get_mut(self.items) };
        let free_list = meta_array.cast::<FreeListItem<S::Handle>>().as_ptr();
        unsafe {
            let bitflags = free_list.cast::<u8>().add(bitflags);
            (
                slice::from_raw_parts_mut(free_list, self.max_size.get()),
                slice::from_raw_parts_mut(bitflags, bitflags_len),
            )
        }
    }
}

unsafe impl<S: SharedGetMut> SharedGetMut for FreeListStorage<S> {
    unsafe fn shared_get_mut(&self, FreeListHandle(handle): Self::Handle) -> core::ptr::NonNull<u8> {
        self.storage.shared_get_mut(handle)
    }
}

unsafe impl<S: Storage> Storage for FreeListStorage<S> {
    type Handle = FreeListHandle<S::Handle>;

    unsafe fn get(&self, FreeListHandle(handle): Self::Handle) -> core::ptr::NonNull<u8> { self.storage.get(handle) }

    unsafe fn get_mut(&mut self, FreeListHandle(handle): Self::Handle) -> core::ptr::NonNull<u8> {
        self.storage.get_mut(handle)
    }

    fn allocate_nonempty(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let (free_list, bitflags) = self.free_list_mut();
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

                        return Ok(NonEmptyMemoryBlock {
                            handle: free_list.handle.get(),
                            size: unsafe { NonZeroUsize::new_unchecked(layout.size()) },
                        })
                    }
                }
            }
        }

        let memory = self.storage.allocate_nonempty(layout)?;
        Ok(NonEmptyMemoryBlock {
            handle: FreeListHandle(memory.handle),
            size: memory.size,
        })
    }

    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout) {
        let (free_list, bitflags) = self.free_list_mut();
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
                    let free_list = free_list.get_unchecked_mut(index);
                    free_list.layout = Cell::new(layout.into());
                    free_list.handle = Cell::new(handle);
                    return
                }
            }
        }

        self.storage.deallocate_nonempty(handle.0, layout)
    }
}

impl<S: SharedStorage> FreeListStorage<S> {
    fn attempt_to_allocate<H: Copy>(
        free_list: &[FreeListItem<H>],
        bitflags: &[AtomicU8],
        layout: NonEmptyLayout,
        was_blocked: &mut bool,
    ) -> Option<NonEmptyMemoryBlock<FreeListHandle<H>>> {
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

    fn attempt_to_deallocate<H: Copy>(
        free_list: &[FreeListItem<H>],
        bitflags: &[AtomicU8],
        handle: FreeListHandle<H>,
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

unsafe impl<S: SharedStorage> SharedStorage for FreeListStorage<S> {
    fn shared_allocate_nonempty(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        let (free_list, bitflags) = self.free_list();

        let waiter = crate::backoff::Backoff::new();
        while waiter.spin() {
            let mut was_blocked = false;
            if let Some(memory_block) = Self::attempt_to_allocate(free_list, bitflags, layout, &mut was_blocked) {
                return Ok(memory_block)
            }
            if !was_blocked {
                break
            }
        }

        let memory = self.storage.shared_allocate_nonempty(layout)?;
        Ok(NonEmptyMemoryBlock {
            handle: FreeListHandle(memory.handle),
            size: memory.size,
        })
    }

    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout) {
        let (free_list, bitflags) = self.free_list();

        let waiter = crate::backoff::Backoff::new();
        while waiter.spin() {
            let mut was_blocked = false;
            if Self::attempt_to_deallocate(free_list, bitflags, handle, layout, &mut was_blocked) {
                return
            }
            if !was_blocked {
                break
            }
        }

        self.storage.shared_deallocate_nonempty(handle.0, layout)
    }
}

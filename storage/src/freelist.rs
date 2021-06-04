use core::{
    alloc::Layout,
    cell::Cell,
    mem::MaybeUninit,
    num::NonZeroUsize,
    slice,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::{
    AllocErr, Handle, NonEmptyLayout, NonEmptyMemoryBlock, OffsetHandle, SharedGetMut, SharedOffsetHandle,
    SharedStorage, Storage,
};

struct FreeListItem<H> {
    owned: AtomicU8,
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

const EMPTY: u8 = 0;
const LOCKED: u8 = 1;
const FILLED: u8 = 2;

unsafe impl<H: Handle> Handle for FreeListHandle<H> {
    unsafe fn dangling(align: usize) -> Self { Self(Handle::dangling(align)) }
}

impl<S: Storage> FreeListStorage<S> {
    pub fn new(max_size: NonZeroUsize, storage: S) -> Self {
        Self::try_new(max_size, storage).unwrap_or_else(AllocErr::handle)
    }

    /// # Panics
    ///
    /// * If layout could not be computed TODO
    pub fn try_new(max_size: NonZeroUsize, mut storage: S) -> Result<Self, AllocErr<S>> {
        let layout = Layout::new::<FreeListItem<S::Handle>>()
            .repeat(max_size.get())
            .unwrap()
            .0;
        let layout = unsafe { NonEmptyLayout::new_unchecked(layout) };
        let meta = match storage.allocate_nonempty(layout) {
            Ok(x) => x.handle,
            Err(err) => return Err(err.with(storage)),
        };
        let meta_array = unsafe { storage.get_mut(meta) };
        let ptr = meta_array.cast::<MaybeUninit<FreeListItem<S::Handle>>>().as_ptr();
        let free_list = unsafe { slice::from_raw_parts_mut(ptr, max_size.get()) };

        for free in free_list {
            *free = MaybeUninit::new(FreeListItem {
                owned: AtomicU8::new(EMPTY),
                layout: Cell::new(Layout::new::<()>()),
                handle: Cell::new(unsafe { Handle::dangling(1) }),
            });
        }

        Ok(Self {
            max_size,
            storage,
            items: meta,
        })
    }
}

impl<S: Storage> FreeListStorage<S> {
    fn free_list(&self) -> &[FreeListItem<S::Handle>] {
        let meta_array = unsafe { self.storage.get(self.items) };
        let ptr = meta_array.cast::<FreeListItem<S::Handle>>().as_ptr();
        unsafe { slice::from_raw_parts(ptr, self.max_size.get()) }
    }

    fn free_list_mut(&mut self) -> &mut [FreeListItem<S::Handle>] {
        let meta_array = unsafe { self.storage.get_mut(self.items) };
        let ptr = meta_array.cast::<FreeListItem<S::Handle>>().as_ptr();
        unsafe { slice::from_raw_parts_mut(ptr, self.max_size.get()) }
    }
}

unsafe impl<S: SharedGetMut + OffsetHandle> SharedGetMut for FreeListStorage<S> {
    unsafe fn shared_get_mut(&self, FreeListHandle(handle): Self::Handle) -> core::ptr::NonNull<u8> {
        self.storage.shared_get_mut(handle)
    }
}

unsafe impl<S: OffsetHandle> Storage for FreeListStorage<S> {
    type Handle = FreeListHandle<S::Handle>;

    unsafe fn get(&self, FreeListHandle(handle): Self::Handle) -> core::ptr::NonNull<u8> { self.storage.get(handle) }

    unsafe fn get_mut(&mut self, FreeListHandle(handle): Self::Handle) -> core::ptr::NonNull<u8> {
        self.storage.get_mut(handle)
    }

    fn allocate_nonempty(
        &mut self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        unsafe {
            for item in self.free_list_mut() {
                let owned = item.owned.get_mut();
                if *owned == FILLED {
                    let item_layout = item.layout.get();

                    if item_layout.align() == layout.align() && item_layout.size() >= layout.size() {
                        *owned = EMPTY;

                        return Ok(NonEmptyMemoryBlock {
                            handle: item.handle.get(),
                            size: NonZeroUsize::new_unchecked(layout.size()),
                        })
                    }
                }
            }

            let memory = self.storage.allocate_nonempty(layout)?;
            Ok(NonEmptyMemoryBlock {
                handle: FreeListHandle(memory.handle),
                size: memory.size,
            })
        }
    }

    unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: NonEmptyLayout) {
        for item in self.free_list_mut() {
            let owned = item.owned.get_mut();
            if *owned == EMPTY {
                *owned = FILLED;
                item.handle.set(handle);
                item.layout.set(layout.into());
                return
            }
        }

        self.storage.deallocate_nonempty(handle.0, layout)
    }
}

unsafe impl<S: SharedOffsetHandle> SharedStorage for FreeListStorage<S> {
    fn shared_allocate_nonempty(
        &self,
        layout: NonEmptyLayout,
    ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, AllocErr> {
        unsafe {
            let waiter = crate::backoff::Backoff::new();

            loop {
                let mut was_blocked = false;
                for item in self.free_list() {
                    let locked = item
                        .owned
                        .compare_exchange(FILLED, LOCKED, Ordering::Acquire, Ordering::Relaxed);
                    was_blocked = was_blocked || locked == Err(LOCKED);
                    if locked.is_ok() {
                        let item_layout = item.layout.get();

                        if item_layout.align() == layout.align() && item_layout.size() >= layout.size() {
                            let handle = item.handle.get();
                            item.owned.store(EMPTY, Ordering::Release);

                            return Ok(NonEmptyMemoryBlock {
                                handle,
                                size: NonZeroUsize::new_unchecked(layout.size()),
                            })
                        }

                        item.owned.store(EMPTY, Ordering::Release);
                    }
                }

                if !was_blocked || !waiter.spin() {
                    break
                }
            }

            let memory_block = self.storage.shared_allocate_nonempty(layout)?;

            Ok(NonEmptyMemoryBlock {
                handle: FreeListHandle(memory_block.handle),
                size: memory_block.size,
            })
        }
    }

    unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: NonEmptyLayout) {
        let waiter = crate::backoff::Backoff::new();

        loop {
            let mut was_blocked = false;
            for item in self.free_list() {
                let locked = item
                    .owned
                    .compare_exchange(EMPTY, LOCKED, Ordering::Acquire, Ordering::Relaxed);
                if locked.is_ok() {
                    item.handle.set(handle);
                    item.layout.set(layout.into());
                    item.owned.store(FILLED, Ordering::Release);
                    return
                }
                was_blocked = was_blocked || locked == Err(LOCKED);
            }
            if !was_blocked || !waiter.spin() {
                break
            }
        }

        self.storage.shared_deallocate_nonempty(handle.0, layout)
    }
}

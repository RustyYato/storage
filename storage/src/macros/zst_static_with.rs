#[doc(hidden)]
#[macro_export]
macro_rules! zst_static_with {
    (
        [[[$($make_storage:tt)*]]]
        [[[$storage:expr]]]
        [[[$token:expr]]]

        $(#[$meta:meta])*
        $v:vis struct $name:ident

        $(#[$handle_meta:meta])*
        with struct $handle:ident

        $(#[resizable = $resizable:meta])?
        as $type:ty
    ) => {
        $(#[$meta])*
        #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $v struct $name;

        $(#[$handle_meta])*
        #[derive(Clone, Copy)]
        $v struct $handle(<$type as $crate::Storage>::Handle);

        const _: () = {
            static TOKEN: $crate::macros::MacroToken = $token;

            $($make_storage)*

            type __InnerHandle = <$type as $crate::Storage>::Handle;

            #[inline]
            fn storage() -> &'static $type {
                $storage
            }

            impl $handle {
                #[inline]
                #[allow(clippy::missing_const_for_fn)]
                fn inner(self) -> __InnerHandle {
                    self.0
                }

                #[inline]
                fn map(result: $crate::macros::MbR<__InnerHandle>) -> $crate::macros::MbR<Self> {
                    $crate::macros::map_mbr(result, Self)
                }

                #[inline]
                fn map_ne(result: $crate::macros::NeMbR<__InnerHandle>) -> $crate::macros::NeMbR<Self> {
                    $crate::macros::map_nembr(result, Self)
                }
            }

            unsafe impl $crate::Handle for $handle {
                #[inline]
                unsafe fn dangling(align: usize) -> Self {
                    Self(<__InnerHandle as $crate::Handle>::dangling(align))
                }
            }

            unsafe impl $crate::PointerHandle for $handle {
                #[inline]
                unsafe fn get(self) -> $crate::macros::core::ptr::NonNull<u8> {
                    $crate::Storage::get(storage(), self.0)
                }

                #[inline]
                unsafe fn get_mut(self) -> $crate::macros::core::ptr::NonNull<u8> {
                    $crate::SharedGetMut::shared_get_mut(storage(), self.0)
                }
            }


            unsafe impl $crate::FromPtr for $name {
                #[inline]
                unsafe fn from_ptr(&self, ptr: $crate::macros::core::ptr::NonNull<u8>, layout: $crate::macros::core::alloc::Layout) -> Self::Handle {
                    $handle($crate::FromPtr::from_ptr(storage(), ptr, layout))
                }
            }

            unsafe impl $crate::SharedGetMut for $name {
                #[inline]
                unsafe fn shared_get_mut(&self, handle: Self::Handle) -> $crate::macros::core::ptr::NonNull<u8> { $crate::PointerHandle::get(handle) }
            }

            unsafe impl $crate::Storage for $name {
                type Handle = $handle;

                #[inline]
                unsafe fn get(&self, handle: Self::Handle) -> $crate::macros::core::ptr::NonNull<u8> { $crate::PointerHandle::get(handle) }

                #[inline]
                unsafe fn get_mut(&mut self, handle: Self::Handle) -> $crate::macros::core::ptr::NonNull<u8> { $crate::PointerHandle::get_mut(handle) }

                #[inline]
                fn allocate_nonempty(
                    &mut self,
                    layout: $crate::NonEmptyLayout,
                ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map_ne($crate::SharedStorage::shared_allocate_nonempty(storage(), layout))
                }

                #[inline]
                unsafe fn deallocate_nonempty(&mut self, handle: Self::Handle, layout: $crate::NonEmptyLayout) {
                    $crate::SharedStorage::shared_deallocate_nonempty(storage(), handle.inner(), layout)
                }

                #[inline]
                fn allocate(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedStorage::shared_allocate(storage(), layout))
                }

                #[inline]
                unsafe fn deallocate(&mut self, handle: Self::Handle, layout: core::alloc::Layout) {
                    $crate::SharedStorage::shared_deallocate(storage(), handle.inner(), layout)
                }

                #[inline]
                fn allocate_nonempty_zeroed(
                    &mut self,
                    layout: $crate::NonEmptyLayout,
                ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map_ne($crate::SharedStorage::shared_allocate_nonempty_zeroed(storage(), layout))
                }

                #[inline]
                fn allocate_zeroed(&mut self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedStorage::shared_allocate_zeroed(storage(), layout))
                }
            }

            $(#[$resizable])?
            unsafe impl $crate::ResizableStorage for $name {
                #[inline]
                unsafe fn grow(
                    &mut self,
                    handle: Self::Handle,
                    old: core::alloc::Layout,
                    new: core::alloc::Layout,
                ) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedResizableStorage::shared_grow(storage(), handle.inner(), old, new))
                }

                #[inline]
                unsafe fn grow_zeroed(
                    &mut self,
                    handle: Self::Handle,
                    old: core::alloc::Layout,
                    new: core::alloc::Layout,
                ) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedResizableStorage::shared_grow_zeroed(storage(), handle.inner(), old, new))
                }

                #[inline]
                unsafe fn shrink(
                    &mut self,
                    handle: Self::Handle,
                    old: core::alloc::Layout,
                    new: core::alloc::Layout,
                ) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedResizableStorage::shared_shrink(storage(), handle.inner(), old, new))
                }
            }

            unsafe impl $crate::SharedStorage for $name {
                #[inline]
                fn shared_allocate_nonempty(
                    &self,
                    layout: $crate::NonEmptyLayout,
                ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map_ne($crate::SharedStorage::shared_allocate_nonempty(storage(), layout))
                }

                #[inline]
                unsafe fn shared_deallocate_nonempty(&self, handle: Self::Handle, layout: $crate::NonEmptyLayout) {
                    $crate::SharedStorage::shared_deallocate_nonempty(storage(), handle.inner(), layout)
                }

                #[inline]
                fn shared_allocate(&self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedStorage::shared_allocate(storage(), layout))
                }

                #[inline]
                unsafe fn shared_deallocate(&self, handle: Self::Handle, layout: core::alloc::Layout) {
                    $crate::SharedStorage::shared_deallocate(storage(), handle.inner(), layout)
                }

                #[inline]
                fn shared_allocate_nonempty_zeroed(
                    &self,
                    layout: $crate::NonEmptyLayout,
                ) -> Result<crate::NonEmptyMemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map_ne($crate::SharedStorage::shared_allocate_nonempty_zeroed(storage(), layout))
                }

                #[inline]
                fn shared_allocate_zeroed(&self, layout: core::alloc::Layout) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedStorage::shared_allocate_zeroed(storage(), layout))
                }
            }

            $(#[$resizable])?
            unsafe impl $crate::SharedResizableStorage for $name {
                #[inline]
                unsafe fn shared_grow(
                    &self,
                    handle: Self::Handle,
                    old: core::alloc::Layout,
                    new: core::alloc::Layout,
                ) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedResizableStorage::shared_grow(storage(), handle.inner(), old, new))
                }

                #[inline]
                unsafe fn shared_grow_zeroed(
                    &self,
                    handle: Self::Handle,
                    old: core::alloc::Layout,
                    new: core::alloc::Layout,
                ) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedResizableStorage::shared_grow_zeroed(storage(), handle.inner(), old, new))
                }

                #[inline]
                unsafe fn shared_shrink(
                    &self,
                    handle: Self::Handle,
                    old: core::alloc::Layout,
                    new: core::alloc::Layout,
                ) -> Result<crate::MemoryBlock<Self::Handle>, $crate::AllocErr> {
                    $handle::map($crate::SharedResizableStorage::shared_shrink(storage(), handle.inner(), old, new))
                }
            }
        };
    };
}

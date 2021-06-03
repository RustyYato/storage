#[macro_export(local_inner_macros)]
macro_rules! install_global_allocator {
    (let GLOBAL: $type:ty = $global:expr $(;)?) => {{
        use $crate::{
            macros::{assert_thread_safe, assume_init_ref, core::mem::MaybeUninit},
            set_global_storage_with,
        };

        {
            static mut GLOBAL: MaybeUninit<$type> = MaybeUninit::uninit();
            const GLOBAL_STORAGE: $crate::GlobalAsPtrStorage<__InstallGlobalStorage> =
                unsafe { $crate::GlobalAsPtrStorage::new(__InstallGlobalStorage) };

            zst_static_with! {
                [[[  ]]]

                [[[
                    unsafe { assume_init_ref(&GLOBAL) }
                ]]]

                [[[
                    unsafe { $crate::macros::MacroToken::new() }
                ]]]

                struct __InstallGlobalStorage
                with struct __InstallGlobalStorageHandle
                as $type
            }

            let _ = assert_thread_safe::<$type>;
            if !set_global_storage_with(|| match MaybeUninit::new($global) {
                global => unsafe {
                    GLOBAL = global;
                    &GLOBAL_STORAGE
                },
            }) {
                $crate::macros::could_not_init()
            }
        }
    }};
}

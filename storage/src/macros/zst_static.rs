#[macro_export(local_inner_macros)]
macro_rules! zst_static {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident

        $(#[$handle_meta:meta])*
        with struct $handle:ident

        $(#[resizable = $resizable:meta])?
        as $type:ty = $value:expr $(;)?
    ) => {
        zst_static_with! {
            [[[
                static __STATIC_STORAGE: $type = $value;
            ]]]

            [[[
                &__STATIC_STORAGE
            ]]]

            [[[
                unsafe { $crate::macros::MacroToken::new() }
            ]]]

            $(#[$meta])*
            $v struct $name
            $(#[$handle_meta])*
            with struct $handle
            $(#[resizable = $resizable])?
            as $type
        }
    };
}

#[macro_export(local_inner_macros)]
macro_rules! zst_runtime {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident

        $(#[$handle_meta:meta])*
        with struct $handle:ident

        $(#[resizable = $resizable:meta])?
        as $type:ty = $value:expr;

        $memory:ident $once:ident
    ) => {
        zst_static_with! {
            [[[]]]

            [[[
                unsafe {
                    $crate::macros::core::assert!($once.is_done());
                    $crate::macros::assume_init_ref(&$memory)
                }
            ]]]

            [[[
                unsafe { $crate::macros::MacroToken::new() }
            ]]]

            $(#[$meta])*
            $v struct $name
            $(#[$handle_meta])*
            with struct $handle
            $(#[resizable = $resizable])?
            as $type
        }

        static mut $memory: $crate::macros::core::mem::MaybeUninit<$type> = $crate::macros::core::mem::MaybeUninit::uninit();
        static mut $once: $crate::macros::Once = $crate::macros::Once::new();

        if let Some(finsher) = unsafe { $once.attempt() } {
            match $value {
                global => unsafe { $memory = $crate::macros::core::mem::MaybeUninit::new(global) }
            }
            finsher.finish()
        }
    };
}

#[macro_export(local_inner_macros)]
macro_rules! zst_static {
    (
        $(#[$meta:meta])*
        $v:vis struct $name:ident

        $(#[$handle_meta:meta])*
        with struct $handle:ident

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
            as $type
        }
    };
}

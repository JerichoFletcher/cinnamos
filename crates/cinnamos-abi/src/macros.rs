#[doc(hidden)]
#[macro_export]
macro_rules! impl_alias {
    ($(#[$attr:meta])* $id:ident = $base:ty) => {
        $(#[$attr])*
        pub struct $id($base);

        impl From<$base> for $id {
            fn from(value: $base) -> Self {
                Self(value)
            }
        }

        impl From<$id> for $base {
            fn from(value: $id) -> Self {
                value.0
            }
        }

        impl core::fmt::Display for $id {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

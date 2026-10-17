pub use ulid::Ulid;

/// Defines a typed identifier newtype wrapping a [`Ulid`].
///
/// Generates `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`,
/// `PartialOrd`, `Ord`, `Default` (new random ULID), `Display`, `FromStr`,
/// and conversions to/from [`Ulid`].
///
/// # Example
///
/// ```
/// use server::define_id;
///
/// define_id!(NotificationId);
///
/// let id = NotificationId::new();
/// assert_eq!(id.to_string().parse::<NotificationId>().unwrap(), id);
/// ```
#[macro_export]
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name($crate::kernel::id::Ulid);

        impl $name {
            /// Creates a new random identifier (ULID).
            pub fn new() -> Self {
                Self($crate::kernel::id::Ulid::new())
            }

            /// The wrapped ULID.
            pub fn inner(self) -> $crate::kernel::id::Ulid {
                self.0
            }

            /// Timestamp (milliseconds since epoch) embedded in the ULID.
            pub fn timestamp_ms(&self) -> u64 {
                self.0.timestamp_ms()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = $crate::kernel::id::ParseUlidError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse::<$crate::kernel::id::Ulid>().map($name)
            }
        }

        impl From<$crate::kernel::id::Ulid> for $name {
            fn from(inner: $crate::kernel::id::Ulid) -> Self {
                Self(inner)
            }
        }

        impl From<$name> for $crate::kernel::id::Ulid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

/// Parse error produced by [`define_id!`] newtypes.
pub type ParseUlidError = ulid::DecodeError;

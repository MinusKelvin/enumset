#![cfg_attr(all(test, feature = "i128"), feature(i128, i128_type))]

//! A library for defining enums that can be used in compact bit sets.
//!
//! It supports enums up to 64 variants on stable Rust, and up to 128 variants on nightly Rust with
//! the `i128` feature enabled.
//!
//! # Defining enums for use with EnumSet
//!
//! Enums to be used with `EnumSet` should be defined through the `enum_set_type!` macro:
//!
//! ```rust
//! # #[macro_use] extern crate enumset;
//! # use enumset::*;
//! enum_set_type! {
//!     /// Documentation for the enum
//!     pub enum Enum {
//!         A, B, C, D, E, F, G,
//!         #[doc(hidden)] __NonExhaustive,
//!     }
//! }
//! # fn main() { }
//! ```
//!
//! # Working with EnumSets
//!
//! EnumSets can be constructed via `EnumSet::new()` like a normal set. In addition,
//! `enum_set_type!` creates operator overloads that allow you to create EnumSets like so:
//!
//! ```rust
//! # #[macro_use] extern crate enumset;
//! # use enumset::*;
//! # enum_set_type! {
//! #      pub enum Enum {
//! #        A, B, C, D, E, F, G,
//! #    }
//! # }
//! # fn main() {
//! let new_set = Enum::A | Enum::C | Enum::G;
//! assert_eq!(new_set.len(), 3);
//! # }
//! ```
//!
//! The `enum_set!` macro also allows you to create constant EnumSets:
//!
//! ```rust
//! # #[macro_use] extern crate enumset;
//! # use enumset::*;
//! # enum_set_type! {
//! #     enum Enum { A, B, C }
//! # }
//! # fn main() {
//! const CONST_SET: EnumSet<Enum> = enum_set!(Enum, Enum::A | Enum::B);
//! assert_eq!(CONST_SET, Enum::A | Enum::B);
//! # }
//! ```
//!
//! Mutable operations on the HashSet otherwise work basically as expected:
//!
//! ```rust
//! # #[macro_use] extern crate enumset;
//! # use enumset::*;
//! # enum_set_type! {
//! #      pub enum Enum {
//! #        A, B, C, D, E, F, G,
//! #    }
//! # }
//! # fn main() {
//! let mut set = EnumSet::new();
//! set.insert(Enum::A);
//! set.insert_all(Enum::E | Enum::G);
//! assert!(set.contains(Enum::A));
//! assert!(!set.contains(Enum::B));
//! assert_eq!(set, Enum::A | Enum::E | Enum::G);
//! # }
//! ```

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::ops::*;

#[doc(hidden)]
pub trait EnumSetType : Copy {
    type Repr: Shl<u8, Output = Self::Repr> + Eq + Not<Output = Self::Repr> +
               Sub<Output = Self::Repr> + BitOr<Output = Self::Repr> +
               BitAnd<Output = Self::Repr> + BitXor<Output = Self::Repr> +
               BitOrAssign + BitAndAssign + Copy + Debug + Ord + Eq + Hash;
    const ZERO: Self::Repr;
    const ONE : Self::Repr;
    const VARIANT_COUNT: u8;

    fn count_ones(val: Self::Repr) -> usize;
    fn into_u8(self) -> u8;
    fn from_u8(val: u8) -> Self;
}

/// An efficient set type for enums created with the `enum_set_type!` macro.
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct EnumSet<T : EnumSetType>(#[doc(hidden)] pub T::Repr);
impl <T : EnumSetType> EnumSet<T> {
    fn mask(bit: u8) -> T::Repr {
        T::ONE << bit
    }
    fn has_bit(&self, bit: u8) -> bool {
        let mask = Self::mask(bit);
        self.0 & mask == mask
    }

    /// Returns an empty set.
    pub fn new() -> Self {
        EnumSet(T::ZERO)
    }

    /// Returns the number of values in this set.
    pub fn len(&self) -> usize {
        T::count_ones(self.0)
    }
    /// Checks if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0 == T::ZERO
    }
    /// Removes all elements from the set.
    pub fn clear(&mut self) {
        self.0 = T::ZERO
    }

    /// Checks if this set shares no elements with another.
    pub fn is_disjoint(&self, other: Self) -> bool {
        self.0 & other.0 == T::ZERO
    }
    /// Checks if all elements in another set are in this set.
    pub fn is_superset(&self, other: Self) -> bool {
        other.0 & self.0 == other.0
    }
    /// Checks if all elements of this set are in another set.
    pub fn is_subset(&self, other: Self) -> bool {
        other.is_superset(*self)
    }

    /// Returns a set containing the union of all elements in both sets.
    pub fn union(&self, other: Self) -> Self {
        EnumSet(self.0 | other.0)
    }
    /// Returns a set containing all elements in common with another set.
    pub fn intersection(&self, other: Self) -> Self {
        EnumSet(self.0 & other.0)
    }
    /// Returns a set with all elements of the other set removed.
    pub fn difference(&self, other: Self) -> Self {
        EnumSet(self.0 & !other.0)
    }
    /// Returns a set with all elements not contained in both sets.
    pub fn symmetrical_difference(&self, other: Self) -> Self {
        EnumSet(self.0 ^ other.0)
    }

    /// Checks whether this set contains a value.
    pub fn contains(&self, value: T) -> bool {
        self.has_bit(value.into_u8())
    }
    /// Adds a value to this set.
    pub fn insert(&mut self, value: T) -> bool {
        let contains = self.contains(value);
        self.0 |= Self::mask(value.into_u8());
        contains
    }
    /// Removes a value from this set.
    pub fn remove(&mut self, value: T) -> bool {
        let contains = self.contains(value);
        self.0 &= !Self::mask(value.into_u8());
        contains
    }

    /// Adds all elements in another set to this one.
    pub fn insert_all(&mut self, other: Self) {
        self.0 |= other.0
    }
    /// Removes all values in another set from this one.
    pub fn remove_all(&mut self, other: Self) {
        self.0 &= !other.0
    }

    pub fn iter(&self) -> EnumSetIter<T> {
        EnumSetIter(*self, 0)
    }
}
impl <T : EnumSetType> IntoIterator for EnumSet<T> {
    type Item = T;
    type IntoIter = EnumSetIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl <T : EnumSetType> Sub<EnumSet<T>> for EnumSet<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        self.difference(other)
    }
}
impl <T : EnumSetType> BitAnd<EnumSet<T>> for EnumSet<T> {
    type Output = Self;
    fn bitand(self, other: Self) -> Self::Output {
        self.intersection(other)
    }
}
impl <T : EnumSetType> BitOr<EnumSet<T>> for EnumSet<T> {
    type Output = Self;
    fn bitor(self, other: Self) -> Self::Output {
        self.union(other)
    }
}
impl <T : EnumSetType> BitXor<EnumSet<T>> for EnumSet<T> {
    type Output = Self;
    fn bitxor(self, other: Self) -> Self::Output {
        self.symmetrical_difference(other)
    }
}
impl <T : EnumSetType> BitOr<T> for EnumSet<T> {
    type Output = Self;
    fn bitor(self, other: T) -> Self::Output {
        EnumSet(self.0 | Self::mask(other.into_u8()))
    }
}

impl <T : EnumSetType + Debug> Debug for EnumSet<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut is_first = true;
        f.write_str("EnumSet(")?;
        for v in self.iter() {
            if !is_first { f.write_str(" | ")?; }
            is_first = false;
            v.fmt(f)?;
        }
        f.write_str(")")?;
        Ok(())
    }
}

/// An iterator for enums created with the `enum_set_type!` macro.
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug)]
pub struct EnumSetIter<T : EnumSetType>(EnumSet<T>, u8);
impl <T : EnumSetType> Iterator for EnumSetIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        while self.1 < T::VARIANT_COUNT {
            let bit = self.1;
            self.1 += 1;
            if self.0.has_bit(bit) {
                return Some(T::from_u8(bit))
            }
        }
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = T::count_ones((self.0).0 & !((T::ONE << self.1) - T::ONE));
        (left, Some(left))
    }
}

#[macro_export]
#[doc(hidden)]
#[cfg(feature = "i128")]
macro_rules! enum_set_type_internal_count_variants {
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $_10:ident $_11:ident $_12:ident $_13:ident $_14:ident $_15:ident $_16:ident $_17:ident
        $_20:ident $_21:ident $_22:ident $_23:ident $_24:ident $_25:ident $_26:ident $_27:ident
        $_30:ident $_31:ident $_32:ident $_33:ident $_34:ident $_35:ident $_36:ident $_37:ident
        $_40:ident $_41:ident $_42:ident $_43:ident $_44:ident $_45:ident $_46:ident $_47:ident
        $_50:ident $_51:ident $_52:ident $_53:ident $_54:ident $_55:ident $_56:ident $_57:ident
        $_60:ident $_61:ident $_62:ident $_63:ident $_64:ident $_65:ident $_66:ident $_67:ident
        $_70:ident $_71:ident $_72:ident $_73:ident $_74:ident $_75:ident $_76:ident $_77:ident
        $_80:ident $_81:ident $_82:ident $_83:ident $_84:ident $_85:ident $_86:ident $_87:ident
        $_90:ident $_91:ident $_92:ident $_93:ident $_94:ident $_95:ident $_96:ident $_97:ident
        $_a0:ident $_a1:ident $_a2:ident $_a3:ident $_a4:ident $_a5:ident $_a6:ident $_a7:ident
        $_b0:ident $_b1:ident $_b2:ident $_b3:ident $_b4:ident $_b5:ident $_b6:ident $_b7:ident
        $_c0:ident $_c1:ident $_c2:ident $_c3:ident $_c4:ident $_c5:ident $_c6:ident $_c7:ident
        $_d0:ident $_d1:ident $_d2:ident $_d3:ident $_d4:ident $_d5:ident $_d6:ident $_d7:ident
        $_e0:ident $_e1:ident $_e2:ident $_e3:ident $_e4:ident $_e5:ident $_e6:ident $_e7:ident
        $_f0:ident $_f1:ident $_f2:ident $_f3:ident $_f4:ident $_f5:ident $_f6:ident $_f7:ident
        $($rest:ident)+
    ) => {
        ENUM_SET___TOO_MANY_ENUM_VARIANTS___MAX_IS_128
    };
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $_10:ident $_11:ident $_12:ident $_13:ident $_14:ident $_15:ident $_16:ident $_17:ident
        $_20:ident $_21:ident $_22:ident $_23:ident $_24:ident $_25:ident $_26:ident $_27:ident
        $_30:ident $_31:ident $_32:ident $_33:ident $_34:ident $_35:ident $_36:ident $_37:ident
        $_40:ident $_41:ident $_42:ident $_43:ident $_44:ident $_45:ident $_46:ident $_47:ident
        $_50:ident $_51:ident $_52:ident $_53:ident $_54:ident $_55:ident $_56:ident $_57:ident
        $_60:ident $_61:ident $_62:ident $_63:ident $_64:ident $_65:ident $_66:ident $_67:ident
        $_70:ident $_71:ident $_72:ident $_73:ident $_74:ident $_75:ident $_76:ident $_77:ident
        $($rest:ident)+
    ) => {
        enum_set_type_internal! { @$next u128 $($args)* }
    };
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $_10:ident $_11:ident $_12:ident $_13:ident $_14:ident $_15:ident $_16:ident $_17:ident
        $_20:ident $_21:ident $_22:ident $_23:ident $_24:ident $_25:ident $_26:ident $_27:ident
        $_30:ident $_31:ident $_32:ident $_33:ident $_34:ident $_35:ident $_36:ident $_37:ident
        $($rest:ident)+
    ) => {
        enum_set_type_internal! { @$next u64 $($args)* }
    };
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $_10:ident $_11:ident $_12:ident $_13:ident $_14:ident $_15:ident $_16:ident $_17:ident
        $($rest:ident)+
    ) => {
        enum_set_type_internal! { @$next u32 $($args)* }
    };
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $($rest:ident)+
    ) => {
        enum_set_type_internal! { @$next u16 $($args)* }
    };
    ($next:ident ($($args:tt)*) $($rest:ident)*) => {
        enum_set_type_internal! { @$next u8 $($args)* }
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(not(feature = "i128"))]
macro_rules! enum_set_type_internal_count_variants {
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $_10:ident $_11:ident $_12:ident $_13:ident $_14:ident $_15:ident $_16:ident $_17:ident
        $_20:ident $_21:ident $_22:ident $_23:ident $_24:ident $_25:ident $_26:ident $_27:ident
        $_30:ident $_31:ident $_32:ident $_33:ident $_34:ident $_35:ident $_36:ident $_37:ident
        $_40:ident $_41:ident $_42:ident $_43:ident $_44:ident $_45:ident $_46:ident $_47:ident
        $_50:ident $_51:ident $_52:ident $_53:ident $_54:ident $_55:ident $_56:ident $_57:ident
        $_60:ident $_61:ident $_62:ident $_63:ident $_64:ident $_65:ident $_66:ident $_67:ident
        $_70:ident $_71:ident $_72:ident $_73:ident $_74:ident $_75:ident $_76:ident $_77:ident
        $($rest:ident)+
    ) => {
        ENUM_SET___TOO_MANY_ENUM_VARIANTS___MAX_IS_64
    };
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $_10:ident $_11:ident $_12:ident $_13:ident $_14:ident $_15:ident $_16:ident $_17:ident
        $_20:ident $_21:ident $_22:ident $_23:ident $_24:ident $_25:ident $_26:ident $_27:ident
        $_30:ident $_31:ident $_32:ident $_33:ident $_34:ident $_35:ident $_36:ident $_37:ident
        $($rest:ident)+
    ) => {
        enum_set_type_internal! { @$next u64 $($args)* }
    };
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $_10:ident $_11:ident $_12:ident $_13:ident $_14:ident $_15:ident $_16:ident $_17:ident
        $($rest:ident)+
    ) => {
        enum_set_type_internal! { @$next u32 $($args)* }
    };
    ($next:ident ($($args:tt)*)
        $_00:ident $_01:ident $_02:ident $_03:ident $_04:ident $_05:ident $_06:ident $_07:ident
        $($rest:ident)+
    ) => {
        enum_set_type_internal! { @$next u16 $($args)* }
    };
    ($next:ident ($($args:tt)*) $($rest:ident)*) => {
        enum_set_type_internal! { @$next u8 $($args)* }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! enum_set_type_internal {
    // Counting functions
    (@ident ($($random:tt)*) $value:expr) => { $value };
    (@count $($value:tt)*) => {
        0u8 $(+ enum_set_type_internal!(@ident ($value) 1u8))*
    };

    // Codegen
    (@body $repr:ident ($(#[$enum_attr:meta])*) ($($vis:tt)*) $enum_name:ident {
        $($(#[$attr:meta])* $variant:ident,)*
    }) => {
        $(#[$enum_attr])* #[repr(u8)]
        #[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug)]
        $($vis)* enum $enum_name {
            $($(#[$attr])* $variant,)*
        }
        impl $crate::EnumSetType for $enum_name {
            type Repr = $repr;
            const ZERO: Self::Repr = 0;
            const ONE : Self::Repr = 1;
            const VARIANT_COUNT: u8 = enum_set_type_internal!(@count $($variant)*);

            fn count_ones(val: Self::Repr) -> usize {
                val.count_ones() as usize
            }
            fn into_u8(self) -> u8 {
                self as u8
            }
            fn from_u8(val: u8) -> Self {
                unsafe { ::std::mem::transmute(val) }
            }
        }
        impl ::std::ops::BitOr<$enum_name> for $enum_name {
            type Output = $crate::EnumSet<$enum_name>;
            fn bitor(self, other: $enum_name) -> Self::Output {
                enum_set!($enum_name, self | other)
            }
        }
    };
}

/// Defines enums which can be used with EnumSet. While attributes and documentation can be
/// attached to the enums, the variants may not contain data.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate enumset;
/// # use enumset::*;
/// enum_set_type! {
///     enum Enum {
///         A, B, C, D, E, F, G
///     }
///
///     /// Documentation
///     pub enum Enum2 {
///         A, B, C, D, E, F, G,
///         #[doc(hidden)] __NonExhaustive,
///     }
/// }
/// # fn main() { }
/// ```
#[macro_export]
macro_rules! enum_set_type {
    ($(#[$enum_attr:meta])* pub enum $enum_name:ident {
        $($(#[$attr:meta])* $variant:ident),* $(,)*
    } $($rest:tt)*) => {
        enum_set_type_internal_count_variants!(body (($(#[$enum_attr])*) (pub) $enum_name {
            $($(#[$attr])* $variant,)*
        }) $($variant)*);
        enum_set_type!($($rest)*);
    };
    ($(#[$enum_attr:meta])* enum $enum_name:ident {
        $($(#[$attr:meta])* $variant:ident),* $(,)*
    } $($rest:tt)*) => {
        enum_set_type_internal_count_variants!(body (($(#[$enum_attr])*) () $enum_name {
            $($(#[$attr])* $variant,)*
        }) $($variant)*);
        enum_set_type!($($rest)*);
    };
    () => { };
}

/// Creates a EnumSet literal, which can be used in const contexts.
///
/// The format is `enum_set_type!(Type, Type::A | Type::B | Type::C)`, where `Type` is the type
/// the enum will contain, and the rest is the variants that the set will contain.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate enumset;
/// # use enumset::*;
/// # enum_set_type! {
/// #     enum Enum { A, B, C }
/// # }
/// # fn main() {
/// const CONST_SET: EnumSet<Enum> = enum_set!(Enum, Enum::A | Enum::B);
/// assert_eq!(CONST_SET, Enum::A | Enum::B);
/// # }
/// ```
#[macro_export]
macro_rules! enum_set {
    ($enum_name:ty, $($value:path)|+) => {
        $crate::EnumSet::<$enum_name>(
            <$enum_name as $crate::EnumSetType>::ZERO
            $(| (<$enum_name as $crate::EnumSetType>::ONE << ($value as u8)))*
        )
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;

    enum_set_type! {
        enum Enum {
            A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
        }
        enum Enum2 {
            A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
        }
    }

    #[cfg(feature = "i128")]
    enum_set_type! {
        enum LargeEnum {
            _00,  _01,  _02,  _03,  _04,  _05,  _06,  _07,
            _10,  _11,  _12,  _13,  _14,  _15,  _16,  _17,
            _20,  _21,  _22,  _23,  _24,  _25,  _26,  _27,
            _30,  _31,  _32,  _33,  _34,  _35,  _36,  _37,
            _40,  _41,  _42,  _43,  _44,  _45,  _46,  _47,
            _50,  _51,  _52,  _53,  _54,  _55,  _56,  _57,
            _60,  _61,  _62,  _63,  _64,  _65,  _66,  _67,
            _70,  _71,  _72,  _73,  _74,  _75,  _76,  _77,
            A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
        }
    }

    macro_rules! test_variants {
        ($enum_name:ident $test_name:ident $($variant:ident,)*) => {
            #[test]
            fn $test_name() {
                let count = enum_set_type_internal!(@count u8 $($variant)*);
                $(
                    assert!(($enum_name::$variant as u8) < count);
                )*
            }
        }
    }

    test_variants! { Enum enum_variant_range_test
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    }

    #[cfg(feature = "i128")]
    test_variants! { LargeEnum large_enum_variant_range_test
        _00,  _01,  _02,  _03,  _04,  _05,  _06,  _07,
        _10,  _11,  _12,  _13,  _14,  _15,  _16,  _17,
        _20,  _21,  _22,  _23,  _24,  _25,  _26,  _27,
        _30,  _31,  _32,  _33,  _34,  _35,  _36,  _37,
        _40,  _41,  _42,  _43,  _44,  _45,  _46,  _47,
        _50,  _51,  _52,  _53,  _54,  _55,  _56,  _57,
        _60,  _61,  _62,  _63,  _64,  _65,  _66,  _67,
        _70,  _71,  _72,  _73,  _74,  _75,  _76,  _77,
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    }

    macro_rules! test_enum {
        ($e:ident, $m:ident) => {
            mod $m {
                use super::*;

                const CONST_SET: EnumSet<$e> = enum_set!($e, $e::A | $e::Y);
                #[test]
                fn const_set() {
                    assert_eq!(CONST_SET.len(), 2);
                    assert!(CONST_SET.contains($e::A));
                    assert!(CONST_SET.contains($e::Y));
                }

                #[test]
                fn basic_add_remove() {
                    let mut set = EnumSet::new();
                    set.insert($e::A);
                    set.insert($e::B);
                    set.insert($e::C);
                    assert_eq!(set, $e::A | $e::B | $e::C);
                    set.remove($e::B);
                    assert_eq!(set, $e::A | $e::C);
                    set.insert($e::D);
                    assert_eq!(set, $e::A | $e::C | $e::D);
                    set.insert_all($e::F | $e::E | $e::G);
                    assert_eq!(set, $e::A | $e::C | $e::D | $e::F | $e::E | $e::G);
                    set.remove_all($e::A | $e::D | $e::G);
                    assert_eq!(set, $e::C | $e::F | $e::E);
                    assert!(!set.is_empty());
                    set.clear();
                    assert!(set.is_empty());
                }

                #[test]
                fn basic_iter_test() {
                    let mut set = EnumSet::new();
                    set.insert($e::A);
                    set.insert($e::B);
                    set.insert($e::C);
                    set.insert($e::E);

                    let mut set_2 = EnumSet::new();
                    let vec: Vec<$e> = set.iter().collect();
                    for val in vec {
                        set_2.insert(val);
                    }
                    assert_eq!(set, set_2);

                    let mut set_3 = EnumSet::new();
                    for val in set {
                        set_3.insert(val);
                    }
                    assert_eq!(set, set_3);
                }

                #[test]
                fn basic_ops_test() {
                    assert_eq!(($e::A | $e::B) | ($e::B | $e::C), $e::A | $e::B | $e::C);
                    assert_eq!(($e::A | $e::B) & ($e::B | $e::C), EnumSet::new() | $e::B);
                    assert_eq!(($e::A | $e::B) ^ ($e::B | $e::C), $e::A | $e::C);
                    assert_eq!(($e::A | $e::B) - ($e::B | $e::C), EnumSet::new() | $e::A);
                }

                #[test]
                fn basic_set_status() {
                    assert!(($e::A | $e::B | $e::C).is_disjoint($e::D | $e::E | $e::F));
                    assert!(!($e::A | $e::B | $e::C | $e::D).is_disjoint($e::D | $e::E | $e::F));
                    assert!(($e::A | $e::B).is_subset($e::A | $e::B | $e::C));
                    assert!(!($e::A | $e::D).is_subset($e::A | $e::B | $e::C));
                }

                #[test]
                fn debug_impl() {
                    assert_eq!(format!("{:?}", $e::A | $e::B | $e::W), "EnumSet(A | B | W)");
                }
            }
        }
    }

    test_enum!(Enum, small_enum);
    #[cfg(feature = "i128")]
    test_enum!(LargeEnum, large_enum);
}

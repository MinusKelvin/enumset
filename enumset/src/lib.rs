#![no_std]
#![forbid(missing_docs)]
// The safety requirement is "use the procedural derive".
#![allow(clippy::missing_safety_doc)]

//! A library for defining enums that can be used in compact bit sets. It supports enums up to 128
//! variants, and has a macro to use these sets in constants.
//!
//! For serde support, enable the `serde` feature.
//!
//! # Defining enums for use with EnumSet
//!
//! Enums to be used with [`EnumSet`] should be defined using `#[derive(EnumSetType)]`:
//!
//! ```rust
//! # use enumset::*;
//! #[derive(EnumSetType, Debug)]
//! pub enum Enum {
//!    A, B, C, D, E, F, G,
//! }
//! ```
//!
//! For more information on more advanced use cases, see the documentation for
//! [`#[derive(EnumSetType)]`](./derive.EnumSetType.html).
//!
//! # Working with EnumSets
//!
//! EnumSets can be constructed via [`EnumSet::new()`] like a normal set. In addition,
//! `#[derive(EnumSetType)]` creates operator overloads that allow you to create EnumSets like so:
//!
//! ```rust
//! # use enumset::*;
//! # #[derive(EnumSetType, Debug)] pub enum Enum { A, B, C, D, E, F, G }
//! let new_set = Enum::A | Enum::C | Enum::G;
//! assert_eq!(new_set.len(), 3);
//! ```
//!
//! All bitwise operations you would expect to work on bitsets also work on both EnumSets and
//! enums with `#[derive(EnumSetType)]`:
//! ```rust
//! # use enumset::*;
//! # #[derive(EnumSetType, Debug)] pub enum Enum { A, B, C, D, E, F, G }
//! // Intersection of sets
//! assert_eq!((Enum::A | Enum::B) & Enum::C, EnumSet::empty());
//! assert_eq!((Enum::A | Enum::B) & Enum::A, Enum::A);
//! assert_eq!(Enum::A & Enum::B, EnumSet::empty());
//!
//! // Symmetric difference of sets
//! assert_eq!((Enum::A | Enum::B) ^ (Enum::B | Enum::C), Enum::A | Enum::C);
//! assert_eq!(Enum::A ^ Enum::C, Enum::A | Enum::C);
//!
//! // Difference of sets
//! assert_eq!((Enum::A | Enum::B | Enum::C) - Enum::B, Enum::A | Enum::C);
//!
//! // Complement of sets
//! assert_eq!(!(Enum::E | Enum::G), Enum::A | Enum::B | Enum::C | Enum::D | Enum::F);
//! ```
//!
//! The [`enum_set!`] macro allows you to create EnumSets in constant contexts:
//!
//! ```rust
//! # use enumset::*;
//! # #[derive(EnumSetType, Debug)] pub enum Enum { A, B, C, D, E, F, G }
//! const CONST_SET: EnumSet<Enum> = enum_set!(Enum::A | Enum::B);
//! assert_eq!(CONST_SET, Enum::A | Enum::B);
//! ```
//!
//! Mutable operations on the [`EnumSet`] otherwise similarly to Rust's builtin sets:
//!
//! ```rust
//! # use enumset::*;
//! # #[derive(EnumSetType, Debug)] pub enum Enum { A, B, C, D, E, F, G }
//! let mut set = EnumSet::new();
//! set.insert(Enum::A);
//! set.insert_all(Enum::E | Enum::G);
//! assert!(set.contains(Enum::A));
//! assert!(!set.contains(Enum::B));
//! assert_eq!(set, Enum::A | Enum::E | Enum::G);
//! ```

use core::cmp::Ordering;
use core::fmt;
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::iter::{FromIterator, Sum};
use core::ops::*;

#[doc(hidden)]
/// Everything in this module is internal API and may change at any time.
pub mod __internal {
    use super::*;

    /// A reexport of core to allow our macros to be generic to std vs core.
    pub use ::core as core_export;

    /// A reexport of serde so there is no requirement to depend on serde.
    #[cfg(feature = "serde")]
    pub use serde2 as serde;

    /// The actual members of EnumSetType. Put here to avoid polluting global namespaces.
    pub unsafe trait EnumSetTypePrivate {
        /// The underlying type used to store the bitset.
        type Repr: EnumSetTypeRepr;
        /// A mask of bits that are valid in the bitset.
        const ALL_BITS: Self::Repr;

        /// Converts an enum of this type into its bit position.
        fn enum_into_u32(self) -> u32;
        /// Converts a bit position into an enum value.
        unsafe fn enum_from_u32(val: u32) -> Self;

        /// Serializes the `EnumSet`.
        ///
        /// This and `deserialize` are part of the `EnumSetType` trait so the procedural derive
        /// can control how `EnumSet` is serialized.
        #[cfg(feature = "serde")]
        fn serialize<S: serde::Serializer>(set: EnumSet<Self>, ser: S) -> Result<S::Ok, S::Error>
        where Self: EnumSetType;
        /// Deserializes the `EnumSet`.
        #[cfg(feature = "serde")]
        fn deserialize<'de, D: serde::Deserializer<'de>>(de: D) -> Result<EnumSet<Self>, D::Error>
        where Self: EnumSetType;
    }
}
#[cfg(feature = "serde")]
use crate::__internal::serde;
use crate::__internal::EnumSetTypePrivate;
#[cfg(feature = "serde")]
use crate::serde::{Deserialize, Serialize};

mod repr;
use crate::repr::EnumSetTypeRepr;

/// The procedural macro used to derive [`EnumSetType`], and allow enums to be used with
/// [`EnumSet`].
///
/// It may be used with any enum with no data fields, at most 127 variants, and no variant
/// discriminators larger than 127.
///
/// # Additional Impls
///
/// In addition to the implementation of `EnumSetType`, this procedural macro creates multiple
/// other impls that are either required for the macro to work, or make the procedural macro more
/// ergonomic to use.
///
/// A full list of traits implemented as is follows:
///
/// * [`Copy`], [`Clone`], [`Eq`], [`PartialEq`] implementations are created to allow `EnumSet`
///   to function properly. These automatic implementations may be suppressed using
///   `#[enumset(no_super_impls)]`, but these traits must still be implemented in another way.
/// * [`PartialEq`], [`Sub`], [`BitAnd`], [`BitOr`], [`BitXor`], and [`Not`] implementations are
///   created to allow the crate to be used more ergonomically in expressions. These automatic
///   implementations may be suppressed using `#[enumset(no_ops)]`.
///
/// # Options
///
/// Options are given with `#[enumset(foo)]` annotations attached to the same enum as the derive.
/// Multiple options may be given in the same annotation using the `#[enumset(foo, bar)]` syntax.
///
/// A full list of options is as follows:
///
/// * `#[enumset(no_super_impls)]` prevents the derive from creating implementations required for
///   [`EnumSet`] to function. When this attribute is specified, implementations of [`Copy`],
///   [`Clone`], [`Eq`], and [`PartialEq`]. This can be useful if you are using a code generator
///   that already derives these traits. These impls should function identically to the
///   automatically derived versions, or unintentional behavior may be a result.
/// * `#[enumset(no_ops)` prevents the derive from implementing any operator traits.
/// * `#[enumset(crate_name = "enumset2")]` may be used to change the name of the `enumset` crate
///   used in the generated code. When the `std` feature is enabled, enumset parses `Cargo.toml`
///   to determine the name of the crate, and this flag is unnecessary.
/// * `#[enumset(repr = "u8")]` may be used to specify the in-memory representation of `EnumSet`s
///   of this enum type. The effects of this are described in [the `EnumSet` documentation under
///   “FFI, Safety and `repr`”][EnumSet#ffi-safety-and-repr]. Allowed types are `u8`, `u16`, `u32`,
///   `u64` and `u128`. If this is not used, then the derive macro will choose a type to best fit
///   the enum, but there are no guarantees about which type will be chosen.
///
/// When the `serde` feature is used, the following features may also be specified. These options
/// may be used (with no effect) when building without the feature enabled:
///
/// * `#[enumset(serialize_repr = "u8")]` may be used to specify the integer type used to serialize
///   the underlying bitset. Any type allowed in the `repr` option may be used in this option.
/// * `#[enumset(serialize_as_list)]` may be used to serialize the bitset as a list of enum
///   variants instead of an integer. This requires [`Deserialize`] and [`Serialize`] be
///   implemented on the enum.
/// * `#[enumset(serialize_deny_unknown)]` causes the generated deserializer to return an error
///   for unknown bits instead of silently ignoring them.
///
/// # Examples
///
/// Deriving a plain EnumSetType:
///
/// ```rust
/// # use enumset::*;
/// #[derive(EnumSetType)]
/// pub enum Enum {
///    A, B, C, D, E, F, G,
/// }
/// ```
///
/// Deriving a sparse EnumSetType:
///
/// ```rust
/// # use enumset::*;
/// #[derive(EnumSetType)]
/// pub enum SparseEnum {
///    A = 10, B = 20, C = 30, D = 127,
/// }
/// ```
///
/// Deriving an EnumSetType without adding ops:
///
/// ```rust
/// # use enumset::*;
/// #[derive(EnumSetType)]
/// #[enumset(no_ops)]
/// pub enum NoOpsEnum {
///    A, B, C, D, E, F, G,
/// }
/// ```
pub use enumset_derive::EnumSetType;

/// The trait used to define enum types that may be used with [`EnumSet`].
///
/// This trait must be impelmented using `#[derive(EnumSetType)]`, is not public API, and its
/// internal structure may change at any time with no warning.
///
/// For full documentation on the procedural derive and its options, see
/// [`#[derive(EnumSetType)]`](./derive.EnumSetType.html).
pub unsafe trait EnumSetType: Copy + Eq + EnumSetTypePrivate {}

/// An [`EnumSetType`] for which [`EnumSet`]s have a guaranteed in-memory representation.
///
/// An implementation of this trait is generated by using
/// [`#[derive(EnumSetType)]`](./derive.EnumSetType.html) with the annotation
/// `#[enumset(repr = "…")]`, where `…` is `u8`, `u16`, `u32`, `u64` or `u128`.
///
/// For any type `T` that implements this trait, the in-memory representation of `EnumSet<T>`
/// is guaranteed to be `Repr`. This guarantee is useful for FFI. See [the `EnumSet` documentation
/// under “FFI, Safety and `repr`”][EnumSet#ffi-safety-and-repr] for an example.
pub unsafe trait EnumSetTypeWithRepr:
    EnumSetType + EnumSetTypePrivate<Repr = <Self as EnumSetTypeWithRepr>::Repr>
{
    /// The guaranteed representation.
    type Repr: EnumSetTypeRepr;
}

/// An efficient set type for enums.
///
/// It is implemented using a bitset stored using the smallest integer that can fit all bits
/// in the underlying enum. In general, an enum variant with a discriminator of `n` is stored in
/// the nth least significant bit (corresponding with a mask of, e.g. `1 << enum as u32`).
///
/// # Numeric representation
///
/// `EnumSet` is internally implemented using integer types, and as such can be easily converted
/// from and to numbers.
///
/// Each bit of the underlying integer corresponds to at most one particular enum variant. If the
/// corresponding bit for a variant is set, it present in the set. Bits that do not correspond to
/// any variant are always unset.
///
/// By default, each enum variant is stored in a bit corresponding to its discriminator. An enum
/// variant with a discriminator of `n` is stored in the `n + 1`th least significant bit
/// (corresponding to a mask of e.g. `1 << enum as u32`).
///
/// # Serialization
///
/// When the `serde` feature is enabled, `EnumSet`s can be serialized and deserialized using
/// the `serde` crate. The exact serialization format can be controlled with additional attributes
/// on the enum type. These attributes are valid regardless of whether the `serde` feature
/// is enabled.
///
/// By default, `EnumSet`s serialize by directly writing out the underlying bitset as an integer
/// of the smallest type that can fit in the underlying enum. You can add a
/// `#[enumset(serialize_repr = "u8")]` attribute to your enum to control the integer type used
/// for serialization. This can be important for avoiding unintentional breaking changes when
/// `EnumSet`s are serialized with formats like `bincode`.
///
/// By default, unknown bits are ignored and silently removed from the bitset. To override thris
/// behavior, you can add a `#[enumset(serialize_deny_unknown)]` attribute. This will cause
/// deserialization to fail if an invalid bit is set.
///
/// In addition, the `#[enumset(serialize_as_list)]` attribute causes the `EnumSet` to be
/// instead serialized as a list of enum variants. This requires your enum type implement
/// [`Serialize`] and [`Deserialize`]. Note that this is a breaking change.
///
/// # FFI, Safety and `repr`
///
/// If an enum type `T` is annotated with [`#[enumset(repr = "R")]`][derive@EnumSetType#options],
/// then several things happen:
///
/// * `T` will implement <code>[EnumSetTypeWithRepr]&lt;Repr = R&gt;</code> in addition to
///   [`EnumSetType`].
/// * The `EnumSet` methods with `repr` in their name, such as [`as_repr`][EnumSet::as_repr] and
///   [`from_repr`][EnumSet::from_repr], will be available for `EnumSet<T>`.
/// * The in-memory representation of `EnumSet<T>` is guaranteed to be `R`.
///
/// That last guarantee makes it sound to send `EnumSet<T>` across an FFI boundary. For example:
///
/// ```
/// # use enumset::*;
/// #
/// # mod ffi_impl {
/// #     // This example “foreign” function is actually written in Rust, but for the sake
/// #     // of example, we'll pretend it's written in C.
/// #     #[no_mangle]
/// #     extern "C" fn some_foreign_function(set: u32) -> u32 {
/// #         set & 0b100
/// #     }
/// # }
/// #
/// extern "C" {
///     // This function is written in C like:
///     // uint32_t some_foreign_function(uint32_t set) { … }
///     fn some_foreign_function(set: EnumSet<MyEnum>) -> EnumSet<MyEnum>;
/// }
///
/// #[derive(Debug, EnumSetType)]
/// #[enumset(repr = "u32")]
/// enum MyEnum { A, B, C }
///
/// let set: EnumSet<MyEnum> = enum_set!(MyEnum::A | MyEnum::C);
///
/// let new_set: EnumSet<MyEnum> = unsafe { some_foreign_function(set) };
/// assert_eq!(new_set, enum_set!(MyEnum::C));
/// ```
///
/// When an `EnumSet<T>` is received via FFI, all bits that don't correspond to an enum variant
/// of `T` must be set to `0`. Behavior is **undefined** if any of these bits are set to `1`.
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct EnumSet<T: EnumSetType> {
    #[doc(hidden)]
    /// This is public due to the [`enum_set!`] macro.
    /// This is **NOT** public API and may change at any time.
    pub __priv_repr: T::Repr,
}
impl<T: EnumSetType> EnumSet<T> {
    // Returns all bits valid for the enum
    #[inline(always)]
    fn all_bits() -> T::Repr {
        T::ALL_BITS
    }

    /// Creates an empty `EnumSet`.
    #[inline(always)]
    pub fn new() -> Self {
        EnumSet { __priv_repr: T::Repr::empty() }
    }

    /// Returns an `EnumSet` containing a single element.
    #[inline(always)]
    pub fn only(t: T) -> Self {
        let mut set = Self::new();
        set.insert(t);
        set
    }

    /// Creates an empty `EnumSet`.
    ///
    /// This is an alias for [`EnumSet::new`].
    #[inline(always)]
    pub fn empty() -> Self {
        Self::new()
    }

    /// Returns an `EnumSet` containing all valid variants of the enum.
    #[inline(always)]
    pub fn all() -> Self {
        EnumSet { __priv_repr: Self::all_bits() }
    }

    /// Total number of bits used by this type. Note that the actual amount of space used is
    /// rounded up to the next highest integer type (`u8`, `u16`, `u32`, `u64`, or `u128`).
    ///
    /// This is the same as [`EnumSet::variant_count`] except in enums with "sparse" variants.
    /// (e.g. `enum Foo { A = 10, B = 20 }`)
    #[inline(always)]
    pub fn bit_width() -> u32 {
        T::Repr::WIDTH - T::ALL_BITS.leading_zeros()
    }

    /// The number of valid variants that this type can contain.
    ///
    /// This is the same as [`EnumSet::bit_width`] except in enums with "sparse" variants.
    /// (e.g. `enum Foo { A = 10, B = 20 }`)
    #[inline(always)]
    pub fn variant_count() -> u32 {
        T::ALL_BITS.count_ones()
    }

    /// Returns the number of elements in this set.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.__priv_repr.count_ones() as usize
    }
    /// Returns `true` if the set contains no elements.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.__priv_repr.is_empty()
    }
    /// Removes all elements from the set.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.__priv_repr = T::Repr::empty()
    }

    /// Returns `true` if `self` has no elements in common with `other`. This is equivalent to
    /// checking for an empty intersection.
    #[inline(always)]
    pub fn is_disjoint(&self, other: Self) -> bool {
        (*self & other).is_empty()
    }
    /// Returns `true` if the set is a superset of another, i.e., `self` contains at least all the
    /// values in `other`.
    #[inline(always)]
    pub fn is_superset(&self, other: Self) -> bool {
        (*self & other).__priv_repr == other.__priv_repr
    }
    /// Returns `true` if the set is a subset of another, i.e., `other` contains at least all
    /// the values in `self`.
    #[inline(always)]
    pub fn is_subset(&self, other: Self) -> bool {
        other.is_superset(*self)
    }

    /// Returns a set containing any elements present in either set.
    #[inline(always)]
    pub fn union(&self, other: Self) -> Self {
        EnumSet { __priv_repr: self.__priv_repr | other.__priv_repr }
    }
    /// Returns a set containing every element present in both sets.
    #[inline(always)]
    pub fn intersection(&self, other: Self) -> Self {
        EnumSet { __priv_repr: self.__priv_repr & other.__priv_repr }
    }
    /// Returns a set containing element present in `self` but not in `other`.
    #[inline(always)]
    pub fn difference(&self, other: Self) -> Self {
        EnumSet { __priv_repr: self.__priv_repr.and_not(other.__priv_repr) }
    }
    /// Returns a set containing every element present in either `self` or `other`, but is not
    /// present in both.
    #[inline(always)]
    pub fn symmetrical_difference(&self, other: Self) -> Self {
        EnumSet { __priv_repr: self.__priv_repr ^ other.__priv_repr }
    }
    /// Returns a set containing all enum variants not in this set.
    #[inline(always)]
    pub fn complement(&self) -> Self {
        EnumSet { __priv_repr: !self.__priv_repr & Self::all_bits() }
    }

    /// Checks whether this set contains a value.
    #[inline(always)]
    pub fn contains(&self, value: T) -> bool {
        self.__priv_repr.has_bit(value.enum_into_u32())
    }

    /// Adds a value to this set.
    ///
    /// If the set did not have this value present, `true` is returned.
    ///
    /// If the set did have this value present, `false` is returned.
    #[inline(always)]
    pub fn insert(&mut self, value: T) -> bool {
        let contains = !self.contains(value);
        self.__priv_repr.add_bit(value.enum_into_u32());
        contains
    }
    /// Removes a value from this set. Returns whether the value was present in the set.
    #[inline(always)]
    pub fn remove(&mut self, value: T) -> bool {
        let contains = self.contains(value);
        self.__priv_repr.remove_bit(value.enum_into_u32());
        contains
    }

    /// Adds all elements in another set to this one.
    #[inline(always)]
    pub fn insert_all(&mut self, other: Self) {
        self.__priv_repr = self.__priv_repr | other.__priv_repr
    }
    /// Removes all values in another set from this one.
    #[inline(always)]
    pub fn remove_all(&mut self, other: Self) {
        self.__priv_repr = self.__priv_repr.and_not(other.__priv_repr);
    }

    /// Iterates the contents of the set in order from the least significant bit to the most
    /// significant bit.
    ///
    /// Note that iterator invalidation is impossible as the iterator contains a copy of this type,
    /// rather than holding a reference to it.
    pub fn iter(&self) -> EnumSetIter<T> {
        EnumSetIter::new(*self)
    }

    /// Iterates the subsets of the set.
    ///
    /// Note that iterator invalidation is impossible as the iterator contains a copy of this type,
    /// rather than holding a reference to it.
    pub fn subsets(&self) -> EnumSetSubsetIter<T> {
        EnumSetSubsetIter::new(*self)
    }

    /// Returns a `T::Repr` representing the elements of this set.
    ///
    /// Unlike the other `as_*` methods, this method is zero-cost and guaranteed not to fail,
    /// panic or truncate any bits.
    ///
    /// In order to use this method, the definition of `T` must have the `#[enumset(repr = "…")]`
    /// annotation.
    #[inline(always)]
    pub fn as_repr(&self) -> <T as EnumSetTypeWithRepr>::Repr
    where T: EnumSetTypeWithRepr {
        self.__priv_repr
    }

    /// Constructs a bitset from a `T::Repr` without checking for invalid bits.
    ///
    /// Unlike the other `from_*` methods, this method is zero-cost and guaranteed not to fail,
    /// panic or truncate any bits, provided the conditions under “Safety” are upheld.
    ///
    /// In order to use this method, the definition of `T` must have the `#[enumset(repr = "…")]`
    /// annotation.
    ///
    /// # Safety
    ///
    /// All bits in the provided parameter `bits` that don't correspond to an enum variant of
    /// `T` must be set to `0`. Behavior is **undefined** if any of these bits are set to `1`.
    #[inline(always)]
    pub unsafe fn from_repr_unchecked(bits: <T as EnumSetTypeWithRepr>::Repr) -> Self
    where T: EnumSetTypeWithRepr {
        Self { __priv_repr: bits }
    }

    /// Constructs a bitset from a `T::Repr`.
    ///
    /// If a bit that doesn't correspond to an enum variant is set, this
    /// method will panic.
    ///
    /// In order to use this method, the definition of `T` must have the `#[enumset(repr = "…")]`
    /// annotation.
    #[inline(always)]
    pub fn from_repr(bits: <T as EnumSetTypeWithRepr>::Repr) -> Self
    where T: EnumSetTypeWithRepr {
        Self::try_from_repr(bits).expect("Bitset contains invalid variants.")
    }

    /// Attempts to constructs a bitset from a `T::Repr`.
    ///
    /// If a bit that doesn't correspond to an enum variant is set, this
    /// method will return `None`.
    ///
    /// In order to use this method, the definition of `T` must have the `#[enumset(repr = "…")]`
    /// annotation.
    #[inline(always)]
    pub fn try_from_repr(bits: <T as EnumSetTypeWithRepr>::Repr) -> Option<Self>
    where T: EnumSetTypeWithRepr {
        let mask = Self::all().__priv_repr;
        if bits.and_not(mask).is_empty() {
            Some(EnumSet { __priv_repr: bits })
        } else {
            None
        }
    }

    /// Constructs a bitset from a `T::Repr`, ignoring invalid variants.
    ///
    /// In order to use this method, the definition of `T` must have the `#[enumset(repr = "…")]`
    /// annotation.
    #[inline(always)]
    pub fn from_repr_truncated(bits: <T as EnumSetTypeWithRepr>::Repr) -> Self
    where T: EnumSetTypeWithRepr {
        let mask = Self::all().as_repr();
        let bits = bits & mask;
        EnumSet { __priv_repr: bits }
    }
}

/// Helper macro for generating conversion functions.
macro_rules! conversion_impls {
    (
        $(for_num!(
            $underlying:ty, $underlying_str:expr,
            $from_fn:ident $to_fn:ident $from_fn_opt:ident $to_fn_opt:ident,
            $from:ident $try_from:ident $from_truncated:ident $from_unchecked:ident,
            $to:ident $try_to:ident $to_truncated:ident
        );)*
    ) => {
        impl <T : EnumSetType> EnumSet<T> {$(
            #[doc = "Returns a `"]
            #[doc = $underlying_str]
            #[doc = "` representing the elements of this set.\n\nIf the underlying bitset will \
                     not fit in a `"]
            #[doc = $underlying_str]
            #[doc = "` or contains bits that do not correspond to an enum variant, this method \
                     will panic."]
            #[inline(always)]
            pub fn $to(&self) -> $underlying {
                self.$try_to().expect("Bitset will not fit into this type.")
            }

            #[doc = "Tries to return a `"]
            #[doc = $underlying_str]
            #[doc = "` representing the elements of this set.\n\nIf the underlying bitset will \
                     not fit in a `"]
            #[doc = $underlying_str]
            #[doc = "` or contains bits that do not correspond to an enum variant, this method \
                     will instead return `None`."]
            #[inline(always)]
            pub fn $try_to(&self) -> Option<$underlying> {
                EnumSetTypeRepr::$to_fn_opt(&self.__priv_repr)
            }

            #[doc = "Returns a truncated `"]
            #[doc = $underlying_str]
            #[doc = "` representing the elements of this set.\n\nIf the underlying bitset will \
                     not fit in a `"]
            #[doc = $underlying_str]
            #[doc = "`, this method will truncate any bits that don't fit or do not correspond \
                     to an enum variant."]
            #[inline(always)]
            pub fn $to_truncated(&self) -> $underlying {
                EnumSetTypeRepr::$to_fn(&self.__priv_repr)
            }

            #[doc = "Constructs a bitset from a `"]
            #[doc = $underlying_str]
            #[doc = "`.\n\nIf a bit that doesn't correspond to an enum variant is set, this \
                     method will panic."]
            #[inline(always)]
            pub fn $from(bits: $underlying) -> Self {
                Self::$try_from(bits).expect("Bitset contains invalid variants.")
            }

            #[doc = "Attempts to constructs a bitset from a `"]
            #[doc = $underlying_str]
            #[doc = "`.\n\nIf a bit that doesn't correspond to an enum variant is set, this \
                     method will return `None`."]
            #[inline(always)]
            pub fn $try_from(bits: $underlying) -> Option<Self> {
                let bits = T::Repr::$from_fn_opt(bits);
                let mask = Self::all().__priv_repr;
                bits.and_then(|bits| if bits.and_not(mask).is_empty() {
                    Some(EnumSet { __priv_repr: bits })
                } else {
                    None
                })
            }

            #[doc = "Constructs a bitset from a `"]
            #[doc = $underlying_str]
            #[doc = "`, ignoring invalid variants."]
            #[inline(always)]
            pub fn $from_truncated(bits: $underlying) -> Self {
                let mask = Self::all().$to_truncated();
                let bits = <T::Repr as EnumSetTypeRepr>::$from_fn(bits & mask);
                EnumSet { __priv_repr: bits }
            }

            #[doc = "Constructs a bitset from a `"]
            #[doc = $underlying_str]
            #[doc = "`, without checking for invalid bits."]
            ///
            /// # Safety
            ///
            /// All bits in the provided parameter `bits` that don't correspond to an enum variant
            /// of `T` must be set to `0`. Behavior is **undefined** if any of these bits are set
            /// to `1`.
            #[inline(always)]
            pub unsafe fn $from_unchecked(bits: $underlying) -> Self {
                EnumSet { __priv_repr: <T::Repr as EnumSetTypeRepr>::$from_fn(bits) }
            }
        )*}
    }
}
conversion_impls! {
    for_num!(u8, "u8",
             from_u8 to_u8 from_u8_opt to_u8_opt,
             from_u8 try_from_u8 from_u8_truncated from_u8_unchecked,
             as_u8 try_as_u8 as_u8_truncated);
    for_num!(u16, "u16",
             from_u16 to_u16 from_u16_opt to_u16_opt,
             from_u16 try_from_u16 from_u16_truncated from_u16_unchecked,
             as_u16 try_as_u16 as_u16_truncated);
    for_num!(u32, "u32",
             from_u32 to_u32 from_u32_opt to_u32_opt,
             from_u32 try_from_u32 from_u32_truncated from_u32_unchecked,
             as_u32 try_as_u32 as_u32_truncated);
    for_num!(u64, "u64",
             from_u64 to_u64 from_u64_opt to_u64_opt,
             from_u64 try_from_u64 from_u64_truncated from_u64_unchecked,
             as_u64 try_as_u64 as_u64_truncated);
    for_num!(u128, "u128",
             from_u128 to_u128 from_u128_opt to_u128_opt,
             from_u128 try_from_u128 from_u128_truncated from_u128_unchecked,
             as_u128 try_as_u128 as_u128_truncated);
    for_num!(usize, "usize",
             from_usize to_usize from_usize_opt to_usize_opt,
             from_usize try_from_usize from_usize_truncated from_usize_unchecked,
             as_usize try_as_usize as_usize_truncated);
}

impl<T: EnumSetType> Default for EnumSet<T> {
    /// Returns an empty set.
    fn default() -> Self {
        Self::new()
    }
}

impl<T: EnumSetType> IntoIterator for EnumSet<T> {
    type Item = T;
    type IntoIter = EnumSetIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<T: EnumSetType> Sum for EnumSet<T> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(EnumSet::empty(), |a, v| a | v)
    }
}
impl<'a, T: EnumSetType> Sum<&'a EnumSet<T>> for EnumSet<T> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(EnumSet::empty(), |a, v| a | *v)
    }
}
impl<T: EnumSetType> Sum<T> for EnumSet<T> {
    fn sum<I: Iterator<Item = T>>(iter: I) -> Self {
        iter.fold(EnumSet::empty(), |a, v| a | v)
    }
}
impl<'a, T: EnumSetType> Sum<&'a T> for EnumSet<T> {
    fn sum<I: Iterator<Item = &'a T>>(iter: I) -> Self {
        iter.fold(EnumSet::empty(), |a, v| a | *v)
    }
}

impl<T: EnumSetType, O: Into<EnumSet<T>>> Sub<O> for EnumSet<T> {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: O) -> Self::Output {
        self.difference(other.into())
    }
}
impl<T: EnumSetType, O: Into<EnumSet<T>>> BitAnd<O> for EnumSet<T> {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, other: O) -> Self::Output {
        self.intersection(other.into())
    }
}
impl<T: EnumSetType, O: Into<EnumSet<T>>> BitOr<O> for EnumSet<T> {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, other: O) -> Self::Output {
        self.union(other.into())
    }
}
impl<T: EnumSetType, O: Into<EnumSet<T>>> BitXor<O> for EnumSet<T> {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, other: O) -> Self::Output {
        self.symmetrical_difference(other.into())
    }
}

impl<T: EnumSetType, O: Into<EnumSet<T>>> SubAssign<O> for EnumSet<T> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: O) {
        *self = *self - rhs;
    }
}
impl<T: EnumSetType, O: Into<EnumSet<T>>> BitAndAssign<O> for EnumSet<T> {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: O) {
        *self = *self & rhs;
    }
}
impl<T: EnumSetType, O: Into<EnumSet<T>>> BitOrAssign<O> for EnumSet<T> {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: O) {
        *self = *self | rhs;
    }
}
impl<T: EnumSetType, O: Into<EnumSet<T>>> BitXorAssign<O> for EnumSet<T> {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: O) {
        *self = *self ^ rhs;
    }
}

impl<T: EnumSetType> Not for EnumSet<T> {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self::Output {
        self.complement()
    }
}

impl<T: EnumSetType> From<T> for EnumSet<T> {
    fn from(t: T) -> Self {
        EnumSet::only(t)
    }
}

impl<T: EnumSetType> PartialEq<T> for EnumSet<T> {
    fn eq(&self, other: &T) -> bool {
        self.__priv_repr == EnumSet::only(*other).__priv_repr
    }
}
impl<T: EnumSetType + Debug> Debug for EnumSet<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut is_first = true;
        f.write_str("EnumSet(")?;
        for v in self.iter() {
            if !is_first {
                f.write_str(" | ")?;
            }
            is_first = false;
            v.fmt(f)?;
        }
        f.write_str(")")?;
        Ok(())
    }
}

#[allow(clippy::derive_hash_xor_eq)] // This impl exists to change trait bounds only.
impl<T: EnumSetType> Hash for EnumSet<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.__priv_repr.hash(state)
    }
}
impl<T: EnumSetType> PartialOrd for EnumSet<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.__priv_repr.partial_cmp(&other.__priv_repr)
    }
}
impl<T: EnumSetType> Ord for EnumSet<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.__priv_repr.cmp(&other.__priv_repr)
    }
}

#[cfg(feature = "serde")]
impl<T: EnumSetType> Serialize for EnumSet<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        T::serialize(*self, serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: EnumSetType> Deserialize<'de> for EnumSet<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer)
    }
}

/// The iterator used by [`EnumSet`]s.
#[derive(Clone, Debug)]
pub struct EnumSetIter<T: EnumSetType> {
    set: EnumSet<T>,
}
impl<T: EnumSetType> EnumSetIter<T> {
    fn new(set: EnumSet<T>) -> EnumSetIter<T> {
        EnumSetIter { set }
    }
}

impl<T: EnumSetType> Iterator for EnumSetIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.set.is_empty() {
            None
        } else {
            let bit = self.set.__priv_repr.trailing_zeros();
            self.set.__priv_repr.remove_bit(bit);
            unsafe { Some(T::enum_from_u32(bit)) }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.set.len();
        (left, Some(left))
    }
}

impl<T: EnumSetType> DoubleEndedIterator for EnumSetIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.set.is_empty() {
            None
        } else {
            let bit = T::Repr::WIDTH - 1 - self.set.__priv_repr.leading_zeros();
            self.set.__priv_repr.remove_bit(bit);
            unsafe { Some(T::enum_from_u32(bit)) }
        }
    }
}

/// The iterator used by [`EnumSet::subsets`].
#[derive(Clone, Debug)]
pub struct EnumSetSubsetIter<T: EnumSetType> {
    set: EnumSet<T>,
    next: EnumSet<T>,
    done: bool,
}

impl<T: EnumSetType> EnumSetSubsetIter<T> {
    fn new(set: EnumSet<T>) -> EnumSetSubsetIter<T> {
        EnumSetSubsetIter { set, next: EnumSet::empty(), done: false }
    }
}

impl<T: EnumSetType> Iterator for EnumSetSubsetIter<T> {
    type Item = EnumSet<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            None
        } else {
            let current = self.next;

            // Carry-rippler trick for iterating subsets of a bitset. To the best of my knowledge,
            // this was first introduced by Marcel van Kervinck on rec.ganes.chess in 1994.
            //
            // If we have a bitset `d` that we want to enumerate the subsets of, and a current
            // subset `n`, we can get the next subset by filling in the irrelevant bits `!d` and
            // then adding 1. This causes carry bits to carry through the irrelevant bits of `n`.
            // We then mask away whatever irrelevant bits remain.
            
            // The full expression is `((n | !d) + 1) & d`, although we can improve this.
            // Since `n` is a subset of `d`, it shares no bits with `!d`. This means we can replace
            // the bitwise or with an add, to get `(n + !d + 1) & d`. `!d + 1` is equal to `-d`
            // under two's complement, so we can just subtract `d` to get `(n - d) & d`.
            let set = self.set.__priv_repr;
            let next = current.__priv_repr.wrapping_sub(set) & set;

            // SAFETY: By the invariants of `EnumSet<T>`, `set` only has valid bits set. Since we
            // mask away the clear bits of `set`, `next` must also have only valid bits set.
            self.next.__priv_repr = next;
            if next.is_empty() {
                self.done = true;
            }

            Some(current)
        }
    }
}

impl<T: EnumSetType> ExactSizeIterator for EnumSetIter<T> {}

impl<T: EnumSetType> Extend<T> for EnumSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|v| {
            self.insert(v);
        });
    }
}

impl<T: EnumSetType> FromIterator<T> for EnumSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = EnumSet::default();
        set.extend(iter);
        set
    }
}

impl<T: EnumSetType> Extend<EnumSet<T>> for EnumSet<T> {
    fn extend<I: IntoIterator<Item = EnumSet<T>>>(&mut self, iter: I) {
        iter.into_iter().for_each(|v| {
            self.insert_all(v);
        });
    }
}

impl<T: EnumSetType> FromIterator<EnumSet<T>> for EnumSet<T> {
    fn from_iter<I: IntoIterator<Item = EnumSet<T>>>(iter: I) -> Self {
        let mut set = EnumSet::default();
        set.extend(iter);
        set
    }
}

/// Creates a EnumSet literal, which can be used in const contexts.
///
/// The syntax used is `enum_set!(Type::A | Type::B | Type::C)`. Each variant must be of the same
/// type, or a error will occur at compile-time.
///
/// This macro accepts trailing `|`s to allow easier use in other macros.
///
/// # Examples
///
/// ```rust
/// # use enumset::*;
/// # #[derive(EnumSetType, Debug)] enum Enum { A, B, C }
/// const CONST_SET: EnumSet<Enum> = enum_set!(Enum::A | Enum::B);
/// assert_eq!(CONST_SET, Enum::A | Enum::B);
/// ```
///
/// This macro is strongly typed. For example, the following will not compile:
///
/// ```compile_fail
/// # use enumset::*;
/// # #[derive(EnumSetType, Debug)] enum Enum { A, B, C }
/// # #[derive(EnumSetType, Debug)] enum Enum2 { A, B, C }
/// let type_error = enum_set!(Enum::A | Enum2::B);
/// ```
#[macro_export]
macro_rules! enum_set {
    ($(|)*) => {
        $crate::EnumSet { __priv_repr: 0 }
    };
    ($value:path $(|)*) => {
        {
            #[allow(deprecated)] let value = $value.__impl_enumset_internal__const_only();
            value
        }
    };
    ($value:path | $($rest:path)|* $(|)*) => {
        {
            #[allow(deprecated)] let value = $value.__impl_enumset_internal__const_only();
            $(#[allow(deprecated)] let value = $rest.__impl_enumset_internal__const_merge(value);)*
            value
        }
    };
}

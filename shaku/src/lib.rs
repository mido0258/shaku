//! Shaku is a compile time dependency injection library. It can be used directly or through
//! integration with application frameworks such as [Rocket] (see
//! [`shaku_rocket`]).
//!
//! # Getting started
//! See the [getting started guide]
//!
//! # Crate features
//! By default shaku is thread-safe and exposes macros, but these can be disabled by opting out of
//! the following features:
//!
//! - `thread_safe`: Requires components to be `Send + Sync`
//! - `derive`: Uses the `shaku_derive` crate to provide proc-macro derives of `Component` and
//!   `Provider`, and the `module` macro.
//!
//! [Rocket]: https://rocket.rs
//! [`shaku_rocket`]: https://crates.io/crates/shaku_rocket
//! [getting started guide]: guide/index.html

// This lint is ignored because proc-macros aren't allowed in statement position
// (at least until 1.45). Removing the main function makes rustdoc think the
// module macro is a statement instead of top-level item.
// This can be removed once the MSRV is at least 1.45.
#![allow(clippy::needless_doctest_main)]

// Modules
#[macro_use]
mod trait_alias;
mod component;
mod module;
mod parameters;
mod provider;

pub mod guide;

// Reexport proc macros
#[cfg(feature = "derive")]
pub use {shaku_derive::module, shaku_derive::Component, shaku_derive::Provider};

// Reexport OnceCell to support lazy components
#[doc(hidden)]
#[cfg(feature = "thread_safe")]
pub use once_cell::sync::OnceCell;
#[doc(hidden)]
#[cfg(not(feature = "thread_safe"))]
pub use once_cell::unsync::OnceCell;

// Expose a flat module structure
pub use crate::{component::*, module::*, provider::*};

#[macro_export]
#[doc(hidden)]

/// Macro to generate HasComponent and HasVariant implementations.
/// This is invoked by the component's linkage macro.

macro_rules! generate_impls_for_component {
    (
        (
            $property:ident,
            $module:ident ($($ty_generics:tt)*),;
            [$($impl_generics:tt)*],
            [$($where_clause:tt)*],
            $component:ty
        ),
        $interface:ty,
        $impl_type:ty
    ) => {
        impl $($impl_generics)* $crate::HasComponent<$interface> for $module $($ty_generics)* $($where_clause)* {
            fn build_component(
                context: &mut $crate::ModuleBuildContext<Self>
            ) -> ::std::sync::Arc<$interface> {
                let component = <Self as $crate::HasVariant<$component, $impl_type>>::build_variant(context);
                component as ::std::sync::Arc<$interface>
            }

            fn resolve(&self) -> ::std::sync::Arc<$interface> {
                let component = <Self as $crate::HasVariant<$component, $impl_type>>::resolve(self);
                component as ::std::sync::Arc<$interface>
            }

            fn resolve_ref(&self) -> &$interface {
                let component = <Self as $crate::HasVariant<$component, $impl_type>>::resolve_ref(self);
                component as &$interface
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
/// Macro to generate only HasVariant implementation.
/// This is used when multiple components implement the same interface.
macro_rules! generate_variant_impl_for_component {
    (
        (
            $property:ident,
            $module:ident ($($ty_generics:tt)*),;
            [$($impl_generics:tt)*],
            [$($where_clause:tt)*],
            $component:ty
        ),
        $interface:ty,
        $impl_type:ty
    ) => {
        // We rely on the manual HasVariant impl generated in expand_module_macro
        // This macro is just a placeholder to satisfy the linkage macro callback
        // but we don't want it to generate HasComponent.
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! generate_single_impl_v2 {
    (
        ($property:ident, $module:ident $($ty_generics:tt)*, [$($impl_generics:tt)*], [$($where_clause:tt)*]),
        $component:ty,
        $interface:ty
    ) => {
        impl $($impl_generics)* $crate::HasVariant<$component, $interface> for $module $($ty_generics)* $($where_clause)* {
            fn build_variant(context: &mut $crate::ModuleBuildContext<Self>) -> ::std::sync::Arc<$interface> {
                let component = context.build_component::<$component>();
                component as ::std::sync::Arc<$interface>
            }

            fn resolve(&self) -> ::std::sync::Arc<$interface> {
                ::std::sync::Arc::clone(&self.$property) as ::std::sync::Arc<$interface>
            }

            fn resolve_ref(&self) -> &$interface {
                &*self.$property
            }
        }

        compile_error!("generate_impls_for_component called!");
        impl $($impl_generics)* $crate::HasComponent<$interface> for $module $($where_clause)* {
            fn build_component(context: &mut $crate::ModuleBuildContext<Self>) -> ::std::sync::Arc<$interface> {
                <Self as $crate::HasVariant<$component, $interface>>::build_variant(context)
            }

            fn resolve(&self) -> ::std::sync::Arc<$interface> {
                <Self as $crate::HasVariant<$component, $interface>>::resolve(self)
            }

            fn resolve_ref(&self) -> &$interface {
                <Self as $crate::HasVariant<$component, $interface>>::resolve_ref(self)
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! generate_no_impls_for_component {
    (
        (
            $property:ident,
            $module:ident ($($ty_generics:tt)*),;
            [$($impl_generics:tt)*],
            [$($where_clause:tt)*],
            $component:ty
        ),
        $interface:ty,
        $impl_type:ty
    ) => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! generate_module_impls {
    // Base case: No more components
    (
        $context:tt,
        [],
        $seen:tt
    ) => {};

    // Recursive step
    (
        $context:tt,
        [
            ($macro_path:path, $args:tt, $generic_args:tt),
            $($rest:tt)*
        ],
        [$($seen:tt)*]
    ) => {
        $macro_path! {
            $crate::consume_component_impl,
            $args,
            $context,
            [ $($rest)* ],
            [ $($seen)* ],
            $generic_args
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! consume_component_impl {
    (
        $args:tt,
        ($($interface:tt)+),
        $impl_type:ty,
        $context:tt,
        [$($rest:tt)*],
        [$($seen:tt)*],
        $generic_args:tt
    ) => {
        // 2. Check if interface is seen
        $crate::if_interface_not_seen! {
            ($($interface)+),
            [ $($seen)* ],
            {
                // Not seen: Generate HasComponent
                $crate::generate_impls_for_component! {
                    $args, $($interface)+, $impl_type
                }

                // Recurse with interface added to seen
                $crate::generate_module_impls! {
                    $context,
                    [ $($rest)* ],
                    [ $($seen)* (($($interface)+)), ]
                }
            },
            {
                // Seen: Skip HasComponent
                // Recurse without adding to seen (already there)
                $crate::generate_module_impls! {
                    $context,
                    [ $($rest)* ],
                    [ $($seen)* ]
                }
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! if_interface_not_seen {
    (($($target:tt)+), [], { $($then:tt)* }, { $($else:tt)* }) => {
        $($then)*
    };
    (($($target:tt)+), [(($($head:tt)+)), $($tail:tt)*], { $($then:tt)* }, { $($else:tt)* }) => {
        $crate::check_if_same_type! {
            ($($target)+), ($($head)+),
            { $($else)* }, // Same type -> Seen -> Execute Else
            {
                // Different type -> Check tail
                $crate::if_interface_not_seen! {
                    ($($target)+), [ $($tail)* ], { $($then)* }, { $($else)* }
                }
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! with_dollar {
    ($macro:path, $($args:tt)*) => {
        $macro! { $($args)*, $ }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! check_if_same_type {
    (($($t1:tt)+), ($($t2:tt)+), { $($then:tt)* }, { $($else:tt)* }) => {
        $crate::with_dollar! { $crate::run_check_if_same_type, ($($t1)+), ($($t2)+), { $($then)* }, { $($else)* } }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! run_check_if_same_type {
    (($($t1:tt)+), ($($t2:tt)+), { $($then:tt)* }, { $($else:tt)* }, $d:tt) => {
        macro_rules! __shaku_check_type {
            ($($t1)+) => { $($then)* };
            ($d($d other:tt)+) => { $($else)* };
        }
        __shaku_check_type! { $($t2)+ }
    };
}


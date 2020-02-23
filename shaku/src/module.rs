use std::sync::Arc;

use crate::{Component, ContainerBuildContext, Interface, ProvidedInterface, Provider};

/// A module represents a group of services. By implementing traits such as
/// [`HasComponent`] on a module, service dependencies are checked at compile
/// time. At runtime, modules hold the components they are associated with.
///
/// Modules are usually created via the [`module`] macro.
///
/// # Example
/// ```
/// use shaku::{module, Component, Interface};
///
/// trait MyComponent: Interface {}
///
/// #[derive(Component)]
/// #[shaku(interface = MyComponent)]
/// struct MyComponentImpl;
/// impl MyComponent for MyComponentImpl {}
///
/// // MyModule implements Module and HasComponent<dyn MyComponent>
/// module! {
///     MyModule {
///         components = [MyComponentImpl],
///         providers = []
///     }
/// }
/// ```
///
/// [`HasComponent`]: trait.HasComponent.html
/// [`module`]: macro.module.html
pub trait Module: Sized + 'static {
    /// Create the module instance by resolving the components this module
    /// provides.
    fn build(context: &mut ContainerBuildContext<Self>) -> Self;
}

/// Indicates that a module contains a component which implements the interface.
pub trait HasComponent<I: Interface + ?Sized>: Module {
    /// The concrete component which implements the interface
    type Impl: Component<Self, Interface = I>;

    /// Get a reference to the stored component. This is used when resolving the
    /// component.
    fn get_ref(&self) -> &Arc<I>;

    /// Get a mutable reference to the stored component. This is used when
    /// resolving the component.
    fn get_mut(&mut self) -> &mut Arc<I>;
}

/// Indicates that a module contains a provider which implements the interface.
pub trait HasProvider<I: ProvidedInterface + ?Sized>: Module {
    /// The concrete provider which implements the interface
    type Impl: Provider<Self, Interface = I>;
}

/// Create a [`Module`] which is associated with some components and providers.
///
/// # Example
/// ```
/// use shaku::{module, Component, Interface};
///
/// trait MyComponent: Interface {}
///
/// #[derive(Component)]
/// #[shaku(interface = MyComponent)]
/// struct MyComponentImpl;
/// impl MyComponent for MyComponentImpl {}
///
/// // MyModule implements Module and HasComponent<dyn MyComponent>
/// module! {
///     MyModule {
///         components = [MyComponentImpl],
///         providers = []
///     }
/// }
/// ```
///
/// [`Module`]: trait.Module.html
#[macro_export]
macro_rules! module {
    {
        $visibility:vis $module:ident {
            components = [
                $($component:ident),* $(,)?
            ],
            providers = [
                $($provider:ident),* $(,)?
            ] $(,)?
        }
    } => {
        #[allow(non_snake_case)]
        $visibility struct $module {
            $(
                // It would be nice to prefix the property with something like
                // "__di_", but macro_rules does not support concatenating
                // idents on stable.
                $component: ::std::sync::Arc<<$component as $crate::Component<Self>>::Interface>
            ),*
        }

        impl $crate::Module for $module {
            fn build(context: &mut $crate::ContainerBuildContext<Self>) -> Self {
                Self { $(
                    $component: context.resolve::<<$component as $crate::Component<Self>>::Interface>()
                ),* }
            }
        }

        $(
        impl $crate::HasComponent<<$component as $crate::Component<Self>>::Interface> for $module {
            type Impl = $component;

            fn get_ref(&self) -> &::std::sync::Arc<<$component as $crate::Component<Self>>::Interface> {
                &self.$component
            }

            fn get_mut(&mut self) -> &mut ::std::sync::Arc<<$component as $crate::Component<Self>>::Interface> {
                &mut self.$component
            }
        }
        )*

        $(
        impl $crate::HasProvider<<$provider as $crate::Provider<Self>>::Interface> for $module {
            type Impl = $provider;
        }
        )*
    };
}
//! Implementation of a `ContainerBuilder`

use std::any::{type_name, TypeId};
use std::collections::HashMap;

use crate::component::{Component, ComponentBuildFn, Interface};
use crate::container::{Container, ContainerBuildContext, RegisteredType};
use crate::result::Result as DIResult;
use crate::Dependency;

/// Build a [Container](struct.Container.html) registering components
/// with or without parameters.
///
/// Once finished, you have to call [build()](struct.ContainerBuilder.html#method.build)
/// to build the associated `Container`. This method can Err if you tried to register
/// invalid values.
///
/// See [module documentation](index.html) or
/// [ContainerBuilder::build()](struct.ContainerBuilder.html#method.build) for more details.
pub struct ContainerBuilder {
    registration_map: HashMap<TypeId, RegisteredType>,
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        ContainerBuilder {
            registration_map: HashMap::new(),
        }
    }
}

impl ContainerBuilder {
    /// Create a new ContainerBuilder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new component with this builder.
    /// If that component was already registered, the old Component is replaced.
    ///
    /// This method returns a mutable [RegisteredType](struct.RegisteredType.html)
    /// allowing to chain calls to
    /// [with_named_parameter()](struct.RegisteredType.html#method.with_named_parameter)
    /// or [with_typed_parameter()](struct.RegisteredType.html#method.with_typed_parameter)
    /// to add parameters to be used to instantiate this Component.
    pub fn register_type<C: Component>(&mut self) -> &mut RegisteredType {
        self.register_lambda::<C::Interface>(
            type_name::<C>(),
            Box::new(C::build),
            C::dependencies(),
        )
    }

    /// Register a new component with this builder.
    /// If that component was already registered, the old Component is replaced.
    ///
    /// This register method is an alternative to implementing [Component].
    /// This may be useful in cases such as using a mock or dynamically choosing the
    /// implementation based on dependencies.
    ///
    /// This method returns a mutable [RegisteredType](struct.RegisteredType.html)
    /// allowing to chain calls to
    /// [with_named_parameter()](struct.RegisteredType.html#method.with_named_parameter)
    /// or [with_typed_parameter()](struct.RegisteredType.html#method.with_typed_parameter)
    /// to add parameters to be used to instantiate this Component.
    ///
    /// [Component]: component/trait.Component.html
    pub fn register_lambda<I: Interface + ?Sized>(
        &mut self,
        component_name: &str,
        build: ComponentBuildFn,
        dependencies: Vec<Dependency>,
    ) -> &mut RegisteredType {
        let interface_type_id = TypeId::of::<I>();

        let registered_type = RegisteredType::new(
            component_name.to_string(),
            interface_type_id,
            build,
            dependencies,
        );

        let old_value = self
            .registration_map
            .insert(interface_type_id, registered_type);
        if let Some(old_value) = old_value {
            warn!(
                "::shaku::ContainerBuilder::register_lambda::warning trait {:?} already had Component '{:?}) registered to it",
                type_name::<I>(),
                old_value.component
            );
        }

        // Return the registration for further configuration
        self.registration_map.get_mut(&interface_type_id).unwrap()
    }

    /// Parse this `ContainerBuilder` content to check if all the registrations are valid.
    /// If so, consume this `ContainerBuilder` to build a [Container]. The
    /// [ContainerBuildContext] struct will be used to build the [Container].
    /// The components are built at this time.
    ///
    /// [Container]: struct.Container.html
    /// [ContainerBuildContext]: struct.ContainerBuildContext.html
    ///
    /// # Errors
    /// The components are built at this time, so any dependency or parameter errors will be
    /// returned here.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use shaku::{Component, Error as DIError, Interface};
    ///
    /// trait Foo: Interface { fn foo(&self); }
    /// trait FooDuplicate: Interface { fn foo(&self) -> String; }
    ///
    /// #[derive(Component)]
    /// #[interface(Foo)]
    /// struct FooImpl;
    ///
    /// #[derive(Component)]
    /// #[interface(FooDuplicate)]
    /// struct FooDuplicateImpl1;
    ///
    /// #[derive(Component)]
    /// #[interface(FooDuplicate)]
    /// struct FooDuplicateImpl2;
    ///
    /// impl Foo for FooImpl { fn foo(&self) { } }
    /// impl FooDuplicate for FooDuplicateImpl1 { fn foo(&self) -> String { "FooDuplicateImpl1".to_string() } }
    /// impl FooDuplicate for FooDuplicateImpl2 { fn foo(&self) -> String { "FooDuplicateImpl2".to_string() } }
    ///
    /// let mut builder = shaku::ContainerBuilder::new();
    ///
    /// // Valid registration
    /// builder.register_type::<FooImpl>();
    ///
    /// let container = builder.build();
    /// assert!(container.is_ok());
    /// let foo = container.unwrap().resolve::<dyn Foo>();
    /// assert!(foo.is_ok());
    ///
    /// // Invalid registration, duplicate => only the latest Component registered is kept
    /// let mut builder = shaku::ContainerBuilder::new();
    /// builder.register_type::<FooDuplicateImpl1>();
    /// builder.register_type::<FooDuplicateImpl2>();
    ///
    /// let container = builder.build();
    /// assert!(container.is_ok());
    /// let mut container = container.unwrap();
    /// let foo = container.resolve::<dyn FooDuplicate>();
    /// assert!(foo.is_ok());
    /// assert_eq!(foo.unwrap().foo(), "FooDuplicateImpl2".to_string());
    /// ```
    pub fn build(self) -> DIResult<Container> {
        ContainerBuildContext::new(self.registration_map).build()
    }
}

use crate::module::{ComponentMap, ParameterMap};
use crate::parameters::ComponentParameters;
use crate::{Component, HasProvider, Interface, Provider, ProviderFn};
use crate::{ComponentFn, Module};
use std::any::{type_name, TypeId};
use std::fmt::{self, Debug};
use std::sync::Arc;
use std::collections::HashMap;

/// Builds a [`Module`] and its associated components. Build context, such as
/// parameters and resolved components, are stored in this struct.
///
/// [`Module`]: trait.Module.html
pub struct ModuleBuildContext<M: Module> {
    resolved_impls: Vec<TypeId>,
    resolved_components: ComponentMap,
    interface_registry: ComponentMap,
    component_fn_overrides: ComponentMap,
    provider_overrides: ComponentMap,
    parameters: ParameterMap,
    submodules: M::Submodules,
    resolve_chain: Vec<ResolveStep>,
    registrations: HashMap<TypeId, Vec<Box<dyn Fn(&mut Self) + Send + Sync>>>,
}

/// Tracks the current resolution chain. Used to detect circular dependencies.
#[derive(PartialEq)]
struct ResolveStep {
    component_type_name: &'static str,
    component_type_id: TypeId,
    interface_type_name: &'static str,
    interface_type_id: TypeId,
}

impl Debug for ResolveStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.component_type_name)
    }
}

/// Wrapper to store resolved components in the `resolved_components` map.
/// This is used to distinguish between different components that implement the same interface.
struct ResolvedComponent<M: Module, C: Component<M>> {
    component: Arc<C::Interface>,
}

// Manual Send/Sync impls might be needed if the compiler can't infer them,
// but Arc<C::Interface> is Send+Sync, so it should be fine.
// We need to ensure ResolvedComponent is Any.

impl<M: Module, C: Component<M>> ResolvedComponent<M, C> {
    fn new(component: Arc<C::Interface>) -> Self {
        Self { component }
    }
}

impl<M: Module> ModuleBuildContext<M> {
    /// Create the build context
    pub(crate) fn new(
        parameters: ParameterMap,
        component_overrides: ComponentMap,
        component_fn_overrides: ComponentMap,
        provider_overrides: ComponentMap,
        submodules: M::Submodules,
    ) -> Self {
        ModuleBuildContext {
            resolved_impls: vec![],
            resolved_components: component_overrides,
            interface_registry: ComponentMap::new(),
            component_fn_overrides,
            provider_overrides,
            parameters,
            submodules,
            resolve_chain: Vec::new(),
            registrations: HashMap::new(),
        }
    }

    /// Access this module's submodules
    pub fn submodules(&self) -> &M::Submodules {
        &self.submodules
    }

    /// Resolve a component by building it if it is not already resolved or
    /// overridden.
    pub fn build_component<C: Component<M>>(&mut self) -> Arc<C::Interface> {
        // First check for overrides (keyed by Interface)
        if let Some(component) = self.resolved_components.get::<Arc<C::Interface>>() {
            return component.clone();
        }

        // Second check resolved components cache (keyed by Component)
        if let Some(wrapper) = self.resolved_components.get::<ResolvedComponent<M, C>>() {
            return wrapper.component.clone();
        }

        // Check for circular dependencies
        if self.resolve_chain.iter().any(|step| step.component_type_id == TypeId::of::<C>()) {
            panic!("Circular dependency detected while resolving {}. Resolution chain: {:?}", type_name::<C>(), self.resolve_chain);
        }

        // Third check overridden component fn set (will be placed into resolved components)
        self.component_fn_overrides
            .remove::<ComponentFn<M, C::Interface>>()
            .map(|component_fn| {
                self.add_resolve_step::<C>();

                // Build the component
                let component = component_fn(self);
                let component: Arc<C::Interface> = Arc::from(component);
                
                self.resolved_components
                    .insert::<Arc<C::Interface>>(Arc::clone(&component));
                self.resolved_components
                    .insert(ResolvedComponent::<M, C>::new(component.clone()));

                // Resolution was successful, pop the component off the chain
                self.resolve_chain.pop();

                component
            })
            .unwrap_or_else(|| self.build::<C>())
    }

    /// Resolve a variant of component by building it if it is not already resolved or
    /// overridden.
    pub fn build_variant<C: Component<M>>(&mut self) -> Arc<C::Interface> {
        let x = self.resolved_impls.contains(&TypeId::of::<C>());

        if x {
            self.build_component::<C>()
        } else {
            self.build::<C>()
        }
    }

    pub fn build<C: Component<M>>(&mut self) -> Arc<C::Interface> {
        self.add_resolve_step::<C>();

        // Build the component
        let params = self
            .parameters
            .remove::<ComponentParameters<C, C::Parameters>>()
            .unwrap_or_default();

        let component = C::build(self, params.value);
        let component: Arc<C::Interface> = Arc::from(component);
        
        self.interface_index::<C>(component.clone());
        C::register_interfaces(self, component.clone());

        // Insert into cache (Component)
        self.resolved_components
            .insert(ResolvedComponent::<M, C>::new(component.clone()));
            
        self.resolved_impls.push(TypeId::of::<C>());
        
        // Resolution was successful, pop the component off the chain
        self.resolve_chain.pop();

        component
    }

    pub fn interface_index<C: Component<M>>(&mut self, iface: Arc<C::Interface>) -> Option<usize> {
        let num = self.interface_registry.get_mut::<Vec<Arc<C::Interface>>>();

        match num {
            None => {
                self.interface_registry
                    .insert::<Vec<Arc<C::Interface>>>(vec![iface]);
                Some(0)
            }
            Some(v) => {
                v.push(iface);
                Some(v.len() - 1)
            }
        }
    }

    pub fn register_interface<I: Interface + ?Sized>(&mut self, iface: Arc<I>) {
        let num = self.interface_registry.get_mut::<Vec<Arc<I>>>();

        match num {
            None => {
                self.interface_registry
                    .insert::<Vec<Arc<I>>>(vec![iface]);
            }
            Some(v) => {
                v.push(iface);
            }
        };
    }
    pub fn collect<C: Interface + ?Sized>(&mut self) -> Vec<Arc<C>> {
        if let Some(registrations) = self.registrations.remove(&TypeId::of::<C>()) {
            for registration in registrations {
                registration(self);
            }
        }

        self.interface_registry
            .get::<Vec<Arc<C>>>()
            .cloned()
            .unwrap_or_else(|| vec![])
    }

    pub fn collectd<C: Component<M>>(&mut self) -> Vec<Arc<C::Interface>> {
        self.interface_registry
            .get::<Vec<Arc<C::Interface>>>()
            .cloned()
            .unwrap_or_else(|| vec![])
    }
    /// Get a provider function from the given provider impl, or an overridden
    /// one if configured during module build.
    pub fn provider_fn<P: Provider<M>>(&self) -> Arc<ProviderFn<M, P::Interface>>
    where
        M: HasProvider<P::Interface>,
    {
        self.provider_overrides
            .get::<Arc<ProviderFn<M, P::Interface>>>()
            .cloned()
            .unwrap_or_else(|| Arc::new(Box::new(P::provide)))
    }

    pub fn register_component<I: Interface + ?Sized, C: Component<M>>(&mut self) {
        let registration = Box::new(|context: &mut Self| {
            context.build_component::<C>();
        });

        self.registrations
            .entry(TypeId::of::<I>())
            .or_insert_with(Vec::new)
            .push(registration);
    }

    fn add_resolve_step<C: Component<M>>(&mut self) {
        let step = ResolveStep {
            component_type_name: type_name::<C>(),
            component_type_id: TypeId::of::<C>(),
            interface_type_name: type_name::<C::Interface>(),
            interface_type_id: TypeId::of::<C::Interface>(),
        };

        // Check for a circular dependency
        if self.resolve_chain.contains(&step) {
            panic!(
                "Circular dependency detected while resolving {}. Resolution chain: {:?}",
                step.interface_type_name, self.resolve_chain
            );
        }

        // Add this component to the chain
        self.resolve_chain.push(step);
    }
}

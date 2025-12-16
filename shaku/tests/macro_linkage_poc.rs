#![allow(dead_code, unused_macros)]
// This POC demonstrates how a component definition can generate a macro
// that exposes its interfaces, which can then be consumed by a module macro.

// 1. The "derive" macro (simulated)
macro_rules! define_component {
    ($name:ident, [$($interface:ty),*]) => {
        struct $name;
        
        // Generate a macro that "returns" the interfaces by calling a callback
        #[macro_export]
        macro_rules! get_interfaces_for_ {
            ($callback:path) => {
                $callback!($name, [$($interface),*]);
            };
        }
        
        // We need a way to link the name `$name` to `get_interfaces_for_`
        // Usually this is done by convention, e.g. paste! { get_interfaces_for_#name }
    };
}

// 2. The "module" macro (simulated)
macro_rules! module_poc {
    (components = [$($component:ident),*]) => {
        // For each component, we want to generate impls for its interfaces.
        // We need to invoke the helper macro for each component.
        
        $(
            // We need to construct the name of the helper macro.
            // In a real proc-macro, we can do string manipulation.
            // Here in declarative macros, we'd need 'paste' crate.
            // For this POC, we'll assume we can call it.
            
            // This is the tricky part: invoking a macro based on an identifier.
            // In proc-macros (like module!), we CAN construct the identifier `get_interfaces_for_MyComponent`.
            // So let's simulate that the proc-macro generates this call:
            
            get_interfaces_for_!(impl_has_component);
        )*
    };
}

// 3. The callback macro that generates the actual code
macro_rules! impl_has_component {
    ($component:ident, [$($interface:ty),*]) => {
        $(
            impl HasComponent<$interface> for MyModule {
                fn resolve(&self) -> $interface {
                    println!("Resolving {} as {}", stringify!($component), stringify!($interface));
                    Default::default()
                }
            }
        )*
    };
}

// Setup traits and module
trait InterfaceA: Default {}
trait InterfaceB: Default {}
impl InterfaceA for usize {}
impl InterfaceB for bool {}

trait HasComponent<I> {
    fn resolve(&self) -> I;
}

struct MyModule;

// Define the component
define_component!(MyComponent, [usize, bool]); 
// In reality: define_component!(MyComponent, [dyn InterfaceA, dyn InterfaceB]);

// Run the module macro
// module_poc!(components = [MyComponent]); 
// The above line is what we want to write.
// It expands to:
// get_interfaces_for_!(impl_has_component);

#[test]
fn test_macro_linkage() {
    // Manually expanding what module_poc! would generate for MyComponent:
    get_interfaces_for_!(impl_has_component);
    
    let module = MyModule;
    let a: usize = module.resolve();
    assert_eq!(a, 0);
}

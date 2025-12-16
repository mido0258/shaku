use shaku::{module, Component, Interface, HasComponent};
use std::sync::Arc;

trait InterfaceA: Interface {}
trait InterfaceB: Interface {}

#[derive(Component)]
#[shaku(interface = InterfaceA, interface = InterfaceB)]
struct MyComponent;

impl InterfaceA for MyComponent {}
impl InterfaceB for MyComponent {}

module! {
    MyModule {
        components = [
            MyComponent
        ],
        providers = [],

    }
}

#[test]
fn test_multi_interface() {
    let module = MyModule::builder().build();
    let _a: Arc<dyn InterfaceA> = HasComponent::resolve(&module);
    let _b: Arc<dyn InterfaceB> = HasComponent::resolve(&module);
    
    // Verify they are the same instance (if possible, though Arc pointers might differ due to trait object fat pointers)
    // But at least they should both resolve.
}

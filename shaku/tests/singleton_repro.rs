use shaku::{module, Component, Interface, HasComponent};
use std::sync::{Arc, RwLock};

trait InterfaceA: Interface {
    fn set_value(&self, value: usize);
}

trait InterfaceB: Interface {
    fn get_value(&self) -> usize;
}

#[derive(Component)]
#[shaku(interface = InterfaceA)]
#[shaku(interface = InterfaceB)]
struct MyComponent {
    #[shaku(default)]
    value: RwLock<usize>,
}

impl InterfaceA for MyComponent {
    fn set_value(&self, value: usize) {
        *self.value.write().unwrap() = value;
    }
}

impl InterfaceB for MyComponent {
    fn get_value(&self) -> usize {
        *self.value.read().unwrap()
    }
}

module! {
    MyModule {
        components = [MyComponent],
        providers = [],
    }
}

#[test]
fn test_singleton_behavior() {
    let module = MyModule::builder().build();
    
    let a: Arc<dyn InterfaceA> = HasComponent::resolve(&module);
    let b: Arc<dyn InterfaceB> = HasComponent::resolve(&module);
    
    a.set_value(42);
    assert_eq!(b.get_value(), 42, "Component should be a singleton shared across interfaces");
}

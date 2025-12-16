use shaku::{module, Component, HasComponent, Interface};

trait MyInterface: Interface {}

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component1;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component2;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component3;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component4;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component5;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component6;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component7;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component8;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component9;

#[derive(Component)]
#[shaku(interface = MyInterface)]
struct Component10;

impl MyInterface for Component1 {}
impl MyInterface for Component2 {}
impl MyInterface for Component3 {}
impl MyInterface for Component4 {}
impl MyInterface for Component5 {}
impl MyInterface for Component6 {}
impl MyInterface for Component7 {}
impl MyInterface for Component8 {}
impl MyInterface for Component9 {}
impl MyInterface for Component10 {}

module! {
    MyModule {
        components = [
            Component1, Component2, Component3, Component4, Component5,
            Component6, Component7, Component8, Component9, Component10
        ],
        providers = []
    }
}

#[test]
fn test_recursion_limit() {
    let module = MyModule::builder().build();
    let _service: std::sync::Arc<dyn MyInterface> = module.resolve();
}

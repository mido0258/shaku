
use shaku::{module, Component, HasComponent, Interface};

trait Presenter: Interface {}

#[derive(Component)]
#[shaku(interface = Presenter)]
struct PresenterA;
impl Presenter for PresenterA {}

#[derive(Component)]
#[shaku(interface = Presenter)]
struct PresenterB;
impl Presenter for PresenterB {}

module! {
    MyModule {
        components = [PresenterA, PresenterB],
        providers = []
    }
}

#[test]
fn test_resolve() {
    let module = MyModule::builder().build();
    let presenter: std::sync::Arc<dyn Presenter> = module.resolve();
}

fn main() {}

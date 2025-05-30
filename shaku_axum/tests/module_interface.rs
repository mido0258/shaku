//! Module interfaces can be used with `Inject` and `InjectProvided`.
//! The module itself would be stored in state as `Arc<dyn MyModule>`.

use shaku::{module, Component, HasComponent, HasProvider, Interface, Provider};
use shaku_axum::{Inject, InjectProvided};

trait MyComponent: Interface {}
trait MyProvider: Send {}

#[derive(Component)]
#[shaku(interface = MyComponent)]
struct MyComponentImpl;
impl MyComponent for MyComponentImpl {}

#[derive(Provider)]
#[shaku(interface = MyProvider)]
struct MyProviderImpl;
impl MyProvider for MyProviderImpl {}

trait MyModule: HasComponent<dyn MyComponent> + HasProvider<dyn MyProvider> {}

module! {
    MyModuleImpl: MyModule {
        components = [MyComponentImpl],
        providers = [MyProviderImpl]
    }
}

#[allow(unused)]
async fn index(
    _component: Inject<dyn MyModule, dyn MyComponent>,
    _provider: InjectProvided<dyn MyModule, dyn MyProvider>,
) {
}

#[test]
fn compiles_ok() {}

#[test]
fn constructs_axum_router() {
    let _ = axum::Router::new().route("/", axum::routing::get(index));
}

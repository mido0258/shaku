use shaku::Component;

trait MyInterface {}

#[derive(Component)]
struct MyComponent;

impl MyInterface for MyComponent {}

fn main() {}

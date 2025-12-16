
#![allow(dead_code, unused_imports)]

use std::fmt::Debug;
use std::any::Any;
use std::sync::Arc;

trait Interface: Any + Send + Sync {}
impl<T: Any + Send + Sync> Interface for T {}

trait HasComponent<I: Interface + ?Sized> {
    fn build(&self) -> Arc<I>;
}

trait RegisterService<E: Debug + Interface>: Debug + Interface {}
impl<E: Debug + Default + Interface> RegisterService<E> for RegisterServiceImpl<E> {}

#[derive(Debug)]
struct RegisterServiceImpl<E: Debug + Default + Interface> {
    e: E,
}

struct MyModule<E: Debug + Default + Interface> {
    e: std::marker::PhantomData<E>,
}

macro_rules! generate_impl {
    (
        (
            $property:ident,
            $module:ident ($($ty_generics:tt)*),;
            [$($impl_generics:tt)*],
            [$($where_clause:tt)*]
        ),
        ($T:ident)
    ) => {
        impl $($impl_generics)* HasComponent<dyn RegisterService<$T>> for $module $($ty_generics)* $($where_clause)* {
            fn build(&self) -> Arc<dyn RegisterService<$T>> {
                panic!()
            }
        }
    }
}

macro_rules! linkage {
    { 
        $callback:path, 
        (
            $property:ident,
            $module:ident ($($ty_generics:tt)*),;
            [$($impl_generics:tt)*],
            [$($where_clause:tt)*]
        ),
        ($T:ident) 
    } => {
        $callback! { 
            (
                $property,
                $module ($($ty_generics)*),;
                [$($impl_generics)*],
                [$($where_clause)*]
            ),
            ($T) 
        }
    }
}

linkage! {
    generate_impl,
    (
        __di_component_0,
        MyModule (<E>),;
        [<E: Debug + Default + Interface>],
        []
    ),
    (E)
}

fn require_has_component<M: HasComponent<dyn RegisterService<E>>, E: Debug + Interface>() {}

fn main() {
    // This should compile if the impl is correct
    // require_has_component::<MyModule<()>, ()>();
}

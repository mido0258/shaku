//! Implementation of the `module` procedural macro

use crate::debug::get_debug_level;

use crate::structures::module::{ComponentItem, ModuleData, Submodule};
use proc_macro2::{Ident, Span, TokenStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Type;
use syn::parse_quote;

pub fn expand_module_macro(module: ModuleData) -> syn::Result<TokenStream> {
    let debug_level = get_debug_level();
    if debug_level > 1 {
        println!("Module data parsed from input: {:#?}", module);
    }

    // Only capture the build context if there is a lazy component
    let capture_build_context = module
        .services
        .components
        .items
        .iter()
        .any(ComponentItem::is_lazy);

    // Build token streams
    let module_struct = module_struct(&module, capture_build_context);
    let module_trait_impl = module_trait(&module);
    let module_builder = module_builder(&module);
    let module_impl = module_impl(&module, capture_build_context);

    // Collect implementations from the interfaces section
    // These components should NOT get HasComponent (they get HasVariant instead)


    // Also collect components with #[interface(...)] attribute
    // Group them by interface type string
    // We also need to track which components are autowired so we don't generate default HasComponent for them
    // AND so we    // Generate implementations for components
    // Generate HasVariant for all components (no HasComponent to avoid conflicts)

    // Collect HasVariant impls separately
    let variant_impls: Vec<TokenStream> = module
        .services
        .components
        .items
        .iter()
        .enumerate()
        .map(|(i, component)| {
            has_variant_impl_for_component(i, component, &module, capture_build_context)
        })
        .collect();

    // Collect tuples for the recursive macro
    let component_macro_args: Vec<TokenStream> = module
        .services
        .components
        .items
        .iter()
        .enumerate()
        .map(|(i, component)| {
            // Construct the linkage macro name
            let component_ty = &component.ty;
            let macro_path = match component_ty {
                syn::Type::Path(type_path) => {
                    let mut path = type_path.path.clone();
                    // Remove generic arguments from the last segment, as macros don't take them
                    if let Some(last_segment) = path.segments.last_mut() {
                        last_segment.arguments = syn::PathArguments::None;
                    }
                    path
                }
                _ => panic!("Component type must be a path"),
            };
            
            let module_name = &module.metadata.identifier;
            let (impl_generics, ty_generics, where_clause) = &module.metadata.generics.split_for_impl();
            let property = generate_name(i, "component", component.ty.span());

            // Extract generic arguments from the component type
            let generic_args = match component_ty {
                syn::Type::Path(type_path) => {
                    if let Some(last_segment) = type_path.path.segments.last() {
                        match &last_segment.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                let args = &args.args;
                                quote! { (#args) }
                            },
                            _ => quote! { () },
                        }
                    } else {
                        quote! { () }
                    }
                },
                _ => quote! { () },
            };

            // Instead of generating the impl directly, we generate a tuple for the recursive macro
            quote! {
                (
                    #macro_path,
                    (
                        #property,
                        #module_name (#ty_generics),; 
                        [#impl_generics], 
                        [#where_clause],
                        #component_ty
                    ),
                    #generic_args
                ),
            }
        })
        .collect();
            
    let has_provider_impls: Vec<TokenStream> = module
        .services
        .providers
        .items
        .iter()
        .enumerate()
        .map(|(i, provider)| has_provider_impl(i, &provider.ty, &module))
        .collect();



    // Blanket implementation of HasComponents
    let blanket_module_name = &module.metadata.identifier;
    let (_, blanket_ty_generics, blanket_where_clause) = module.metadata.generics.split_for_impl();
    
    let mut impl_generics_obj = module.metadata.generics.clone();
    impl_generics_obj.params.push(parse_quote!(I: ::shaku::Interface + ?Sized));
    let (impl_generics, _, _) = impl_generics_obj.split_for_impl();

    let has_interfaces_impls: Vec<TokenStream> = vec![quote! {
        impl #impl_generics ::shaku::HasComponents<I> for #blanket_module_name #blanket_ty_generics #blanket_where_clause {
            fn collect(context: &mut ::shaku::ModuleBuildContext<Self>) -> Vec<::std::sync::Arc<I>> {
                context.collect::<I>()
            }
        }
    }];



    let has_subcomponent_impls: Vec<TokenStream> = module
        .submodules
        .iter()
        .enumerate()
        .flat_map(|(i, submodule)| {
            submodule
                .services
                .components
                .items
                .iter()
                .map(|component| has_subcomponent_impl(i, submodule, &component.ty, &module))
                .collect::<Vec<_>>()
        })
        .collect();

    let has_subprovider_impls: Vec<TokenStream> = module
        .submodules
        .iter()
        .enumerate()
        .flat_map(|(i, submodule)| {
            submodule
                .services
                .providers
                .items
                .iter()
                .map(|provider| has_subprovider_impl(i, submodule, &provider.ty, &module))
                .collect::<Vec<_>>()
        })
        .collect();

    // Combine token streams for the final macro output
    let output = quote! {
        #module_struct
        #module_trait_impl
        #module_builder
        #module_impl
        #(#variant_impls)*
        
        ::shaku::generate_module_impls! {
            (),
            [
                #(#component_macro_args)*
            ],
            []
        }

        #(#has_interfaces_impls)*
        #(#has_provider_impls)*
        #(#has_subcomponent_impls)*
        #(#has_subprovider_impls)*
    };



    if debug_level > 0 {
        println!("{}", output);
    }

    Ok(output)
}

/// Create the module struct
fn module_struct(module: &ModuleData, capture_build_context: bool) -> TokenStream {
    let component_properties: Vec<TokenStream> = module
        .services
        .components
        .items
        .iter()
        .enumerate()
        .map(|(i, component)| component_property(i, component))
        .collect();

    let provider_properties: Vec<TokenStream> = module
        .services
        .providers
        .items
        .iter()
        .enumerate()
        .map(|(i, provider)| provider_property(i, &provider.ty))
        .collect();


    let submodule_properties: Vec<TokenStream> = module
        .submodules
        .iter()
        .enumerate()
        .map(|(i, sub)| submodule_property(i, sub))
        .collect();

    let visibility = &module.metadata.visibility;
    let module_name = &module.metadata.identifier;
    let module_generics = &module.metadata.generics;
    let where_clause = &module.metadata.generics.where_clause;

    let build_context_property = if capture_build_context {
        quote! { build_context: ::std::sync::Mutex<::shaku::ModuleBuildContext<Self>>, }
    } else {
        TokenStream::new()
    };

    quote! {
        #visibility struct #module_name #module_generics #where_clause {
            #(#component_properties,)*
            #(#provider_properties,)*

            #(#submodule_properties,)*
            #build_context_property
        }
    }
}

/// Create an `impl $module_trait for $module` if there is a module trait
fn module_trait(module: &ModuleData) -> Option<TokenStream> {
    let module_trait = module.metadata.interface.as_ref()?;
    let module_name = &module.metadata.identifier;
    let (impl_generics, ty_generics, where_clause) = module.metadata.generics.split_for_impl();

    Some(quote! {
        impl #impl_generics #module_trait for #module_name #ty_generics #where_clause {}
    })
}

/// Create a Module impl
fn module_impl(module: &ModuleData, capture_build_context: bool) -> TokenStream {
    let module_name = &module.metadata.identifier;
    let (impl_generics, ty_generics, where_clause) = module.metadata.generics.split_for_impl();

    let component_builders: Vec<TokenStream> = module
        .services
        .components
        .items
        .iter()
        .enumerate()
        .map(|(i, component)| component_build(i, component))
        .collect();

    let component_registrations: Vec<TokenStream> = module
        .services
        .components
        .items
        .iter()
        .map(|component| {
            let component_ty = &component.ty;
            let interface = default_interface_from_component(component_ty);
            quote! {
                context.register_component::<#interface, #component_ty>();
            }
        })
        .collect();

    let provider_builders: Vec<TokenStream> = module
        .services
        .providers
        .items
        .iter()
        .enumerate()
        .map(|(i, provider)| provider_build(i, &provider.ty))
        .collect();



    let submodules_init = submodules_init(&module.submodules);
    let submodule_names = submodule_names(&module.submodules);
    let submodule_types: Vec<&Type> = module.submodules.iter().map(|sub| &sub.ty).collect();
    let build_context_init = if capture_build_context {
        quote! { build_context: ::std::sync::Mutex::new(context), }
    } else {
        TokenStream::new()
    };

    quote! {
        impl #impl_generics ::shaku::Module for #module_name #ty_generics #where_clause {
            #[allow(bare_trait_objects)]
            type Submodules = (#(::std::sync::Arc<#submodule_types>),*);

            fn build(mut context: ::shaku::ModuleBuildContext<Self>) -> Self {
                #(#component_registrations)*
                #submodules_init

                Self {
                    #(#component_builders,)*
                    #(#provider_builders,)*

                    #(#submodule_names,)*
                    #build_context_init
                }
            }
        }
    }
}

/// Create the `builder` function on the generated module type
fn module_builder(module: &ModuleData) -> TokenStream {
    let module_name = &module.metadata.identifier;
    let visibility = &module.metadata.visibility;
    let submodule_names = submodule_names(&module.submodules);
    let submodule_types: Vec<&Type> = module.submodules.iter().map(|s| &s.ty).collect();
    let (impl_generics, ty_generics, where_clause) = module.metadata.generics.split_for_impl();

    quote! {
        impl #impl_generics #module_name #ty_generics #where_clause {
            #[allow(bare_trait_objects)]
            #visibility fn builder(
                #(#submodule_names: ::std::sync::Arc<#submodule_types>),*
            ) -> ::shaku::ModuleBuilder<Self> {
                ::shaku::ModuleBuilder::with_submodules((#(#submodule_names),*))
            }
        }
    }
}


/// Create a property initializer for the provider during module build
fn provider_build(index: usize, provider_ty: &Type) -> TokenStream {
    let property = generate_name(index, "provider", provider_ty.span());

    quote! {
        #property: context.provider_fn::<#provider_ty>()
    }
}



/// Create a list of statements to initialize the submodule variables during module build
fn submodules_init(submodules: &Punctuated<Submodule, syn::Token![,]>) -> TokenStream {
    if submodules.is_empty() {
        return TokenStream::new();
    }

    let names = submodule_names(submodules);

    quote! {
        let (#(#names),*) = context.submodules();
        #(
        let #names = ::std::sync::Arc::clone(#names);
        )*
    }
}

/// Create the property which holds a component instance
fn component_property(index: usize, component: &ComponentItem) -> TokenStream {
    let property = generate_name(index, "component", component.ty.span());
    let interface = interface_from_component(&component.ty);

    if component.is_lazy() {
        quote! {
            #property: ::shaku::OnceCell<::std::sync::Arc<#interface>>
        }
    } else {
        quote! {
            #property: ::std::sync::Arc<#interface>
        }
    }
}



/// Create the property which holds a provider function
fn provider_property(index: usize, provider_ty: &Type) -> TokenStream {
    let property = generate_name(index, "provider", provider_ty.span());
    let interface = interface_from_provider(provider_ty);

    quote! {
        #property: ::std::sync::Arc<::shaku::ProviderFn<Self, #interface>>
    }
}

/// Create the property which holds a submodule instance
fn submodule_property(index: usize, submodule: &Submodule) -> TokenStream {
    let property = generate_name(index, "submodule", submodule.ty.span());
    let submodule_ty = &submodule.ty;

    quote! {
        #[allow(bare_trait_objects)]
        #property: ::std::sync::Arc<#submodule_ty>
    }
}







/// Create a HasVariant impl for a component
fn has_variant_impl_for_component(index: usize, component: &ComponentItem, module: &ModuleData, _capture_build_context: bool) -> TokenStream {
    let component_ty = &component.ty;
    let property = generate_name(index, "component", component_ty.span());
    let interface = default_interface_from_component(component_ty);
    let module_name = &module.metadata.identifier;
    let (impl_generics, ty_generics, where_clause) = module.metadata.generics.split_for_impl();

    let get_ref_code = if component.is_lazy() {
        quote! {
            let component = self.#property.get_or_init(|| {
                let mut context = self.build_context.lock().unwrap();
                <Self as ::shaku::HasVariant<#component_ty, #interface>>::build_variant(&mut *context)
            });
        }
    } else {
        quote! { let component = &self.#property; }
    };

    quote! {
        impl #impl_generics ::shaku::HasVariant<#component_ty, #interface> for #module_name #ty_generics #where_clause {
            fn build_variant(
                context: &mut ::shaku::ModuleBuildContext<Self>
            ) -> ::std::sync::Arc<#interface> {
                let component = context.build_component::<#component_ty>();
                component as ::std::sync::Arc<#interface>
            }

            fn resolve(&self) -> ::std::sync::Arc<#interface> {
                #get_ref_code
                ::std::sync::Arc::clone(component) as ::std::sync::Arc<#interface>
            }

            fn resolve_ref(&self) -> &#interface {
                #get_ref_code
                &**component
            }
        }
    }
}

/// Create a HasComponent impl that delegates to HasVariant



/// Create a HasProvider impl
fn has_provider_impl(index: usize, provider_ty: &Type, module: &ModuleData) -> TokenStream {
    let property = generate_name(index, "provider", provider_ty.span());
    let interface = interface_from_provider(provider_ty);
    let module_name = &module.metadata.identifier;
    let (impl_generics, ty_generics, where_clause) = module.metadata.generics.split_for_impl();

    quote! {
        impl #impl_generics ::shaku::HasProvider<#interface> for #module_name #ty_generics #where_clause {
            fn provide(&self) -> ::std::result::Result<
                ::std::boxed::Box<#interface>,
                ::std::boxed::Box<dyn ::std::error::Error>
            > {
                (self.#property)(self)
            }
        }
    }
}

/// Create a HasComponent impl for a subcomponent
fn has_subcomponent_impl(
    submodule_index: usize,
    submodule: &Submodule,
    component_ty: &Type,
    module: &ModuleData,
) -> TokenStream {
    let module_name = &module.metadata.identifier;
    let submodule_ty = &submodule.ty;
    let submodule_names = submodule_names(&module.submodules);
    let submodule_name = generate_name(submodule_index, "submodule", submodule_ty.span());
    let (impl_generics, ty_generics, where_clause) = module.metadata.generics.split_for_impl();
    let interface = component_ty;

    quote! {
        #[allow(bare_trait_objects)]
        impl #impl_generics ::shaku::HasComponent<#interface> for #module_name #ty_generics #where_clause {
            fn build_component(
                context: &mut ::shaku::ModuleBuildContext<Self>
            ) -> ::std::sync::Arc<#interface> {
                let (#(#submodule_names),*) = context.submodules();
                #submodule_name.resolve()
            }

            fn resolve(&self) -> ::std::sync::Arc<#interface> {
                self.#submodule_name.resolve()
            }

            fn resolve_ref(&self) -> &#interface {
                self.#submodule_name.resolve_ref()
            }
        }
    }
}

/// Create a HasProvider impl for a subprovider
fn has_subprovider_impl(
    submodule_index: usize,
    submodule: &Submodule,
    provider_ty: &Type,
    module: &ModuleData,
) -> TokenStream {
    let module_name = &module.metadata.identifier;
    let submodule_ty = &submodule.ty;
    let submodule_name = generate_name(submodule_index, "submodule", submodule_ty.span());
    let (impl_generics, ty_generics, where_clause) = module.metadata.generics.split_for_impl();

    quote! {
        #[allow(bare_trait_objects)]
        impl #impl_generics ::shaku::HasProvider<#provider_ty> for #module_name #ty_generics #where_clause {
            fn provide(&self) -> ::std::result::Result<
                ::std::boxed::Box<#provider_ty>,
                ::std::boxed::Box<dyn ::std::error::Error>
            > {
                ::shaku::HasProvider::provide(::std::sync::Arc::as_ref(&self.#submodule_name))
            }
        }
    }
}

/// Get the interface type of a component via projection
fn interface_from_component(component_ty: &Type) -> TokenStream {
    quote! {
        <#component_ty as ::shaku::Component<Self>>::Interface
    }
}

/// Get the default interface type of a component via DefaultInterface projection
fn default_interface_from_component(component_ty: &Type) -> TokenStream {
    quote! {
        <#component_ty as ::shaku::DefaultInterface>::Interface
    }
}

/// Get the interface type of a provider via projection
fn interface_from_provider(provider_ty: &Type) -> TokenStream {
    quote! {
        <#provider_ty as ::shaku::Provider<Self>>::Interface
    }
}

/// Generate HasComponents for autowired interfaces


/// Generate HasComponent for autowired interfaces (singular)


/// Generate HasVariant for an autowired component

/// Create a property initializer for the component during module build
fn component_build(index: usize, component: &ComponentItem) -> TokenStream {
    let property = generate_name(index, "component", component.ty.span());

    if component.is_lazy() {
        quote! {
            #property: ::shaku::OnceCell::new()
        }
    } else {
        // Use build_component to get Arc<C> directly
        let component_ty = &component.ty;
        quote! {
            #property: context.build_component::<#component_ty>()
        }
    }
}

/// Generate a list of idents to use for the submodules
fn submodule_names(submodules: &Punctuated<Submodule, syn::Token![,]>) -> Vec<Ident> {
    submodules
        .iter()
        .enumerate()
        .map(|(i, sub)| generate_name(i, "submodule", sub.ty.span()))
        .collect()
    }

/// Generate an identifier for a module property.
fn generate_name(index: usize, category: &str, span: Span) -> Ident {
    syn::Ident::new(&format!("__di_{}_{}", category, index), span)
}

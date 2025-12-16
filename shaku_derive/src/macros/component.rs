//! Implementation of the `#[derive(Component)]` procedural macro

use crate::debug::get_debug_level;
use crate::macros::common_output::create_dependency;
use crate::structures::service::{Property, PropertyDefault, PropertyType, ServiceData};
use proc_macro2::{Group, TokenStream, TokenTree};
use syn::{DeriveInput, Ident, Visibility};

pub fn expand_derive_component(input: &DeriveInput) -> syn::Result<TokenStream> {

fn replace_generics(tokens: TokenStream, generics: &[Ident], lifetimes: &[Ident]) -> TokenStream {
    let mut iter = tokens.into_iter().peekable();
    let mut new_tokens = TokenStream::new();
    
    while let Some(token) = iter.next() {
        match token {
            TokenTree::Group(group) => {
                let inner = replace_generics(group.stream(), generics, lifetimes);
                let mut new_group = Group::new(group.delimiter(), inner);
                new_group.set_span(group.span());
                new_tokens.extend(std::iter::once(TokenTree::Group(new_group)));
            }
            TokenTree::Ident(ident) => {
                if generics.contains(&ident) {
                    new_tokens.extend(quote::quote! { $#ident });
                } else {
                    new_tokens.extend(std::iter::once(TokenTree::Ident(ident)));
                }
            }
            TokenTree::Punct(ref p) if p.as_char() == '\'' => {
                // Check if next is ident and is in lifetimes
                if let Some(TokenTree::Ident(ident)) = iter.peek() {
                    if lifetimes.contains(ident) {
                        // Consume the ident
                        let ident = match iter.next() { Some(TokenTree::Ident(i)) => i, _ => unreachable!() };
                        // Emit $ident (which captures the lifetime)
                        new_tokens.extend(quote::quote! { $#ident });
                        continue;
                    }
                }
                new_tokens.extend(std::iter::once(token));
            }
            _ => new_tokens.extend(std::iter::once(token)),
        }
    }
    new_tokens
}
    let service = ServiceData::from_derive_input(input)?;

    let debug_level = get_debug_level();
    if debug_level > 1 {
        println!("Service data parsed from Component input: {:#?}", service);
    }

    let resolve_properties: Vec<TokenStream> = service
        .properties
        .iter()
        .map(create_resolve_property)
        .collect();

    let dependencies: Vec<TokenStream> = service
        .properties
        .iter()
        .filter_map(create_dependency)
        .collect();

    let visibility = &service.metadata.visibility;
    let parameters_properties: Vec<TokenStream> = service
        .properties
        .iter()
        .filter_map(|property| create_parameters_property(property, visibility))
        .collect();

    let parameters_defaults: Vec<TokenStream> = service
        .properties
        .iter()
        .filter_map(|property| create_parameters_default(property, &service.metadata.identifier))
        .collect();

    // Component implementation
    let component_name = service.metadata.identifier;
    let parameters_name = format_ident!("{}Parameters", component_name);
    let parameters_doc = format!(" Parameters for {}", component_name);
    let interfaces = &service.metadata.interfaces;
    let interface = interfaces.first().expect("At least one interface is required");
    let (generic_impls_params, generic_tys, generic_where) = service.metadata.generics.split_for_impl();
    let generic_impls_no_parens = &service.metadata.generics.params;

    // Generate the linkage macro
    let macro_name = component_name.clone();
    let macro_name_internal = format_ident!("__shaku_interfaces_{}_internal", component_name);
    let _linkage_interfaces = if interfaces.len() > 1 {
        interfaces.iter().collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let generic_params: Vec<Ident> = service.metadata.generics.params.iter().map(|p| match p {
        syn::GenericParam::Type(t) => t.ident.clone(),
        syn::GenericParam::Const(c) => c.ident.clone(),
        syn::GenericParam::Lifetime(l) => l.lifetime.ident.clone(),
    }).collect();

    let generic_lifetimes: Vec<Ident> = service.metadata.generics.params.iter().filter_map(|p| match p {
        syn::GenericParam::Lifetime(l) => Some(l.lifetime.ident.clone()),
        _ => None,
    }).collect();

    let transformed_interfaces: Vec<TokenStream> = interfaces.iter().map(|interface| {
        let interface_tokens = quote! { #interface };
        replace_generics(interface_tokens, &generic_params, &generic_lifetimes)
    }).collect();
    
    let impl_type = if interfaces.len() > 1 {
        quote! { #component_name #generic_tys }
    } else {
        let interface = interfaces.first().unwrap();
        let interface_tokens = quote! { dyn #interface };
        replace_generics(interface_tokens, &generic_params, &generic_lifetimes)
    };

    let (macro_pattern, macro_body) = if service.metadata.no_resolve {
        if generic_params.is_empty() {
             (
                quote! { {$callback:path, $args:tt, $context:tt, $rest:tt, $seen:tt, $extra:tt} },
                quote! { }
            )
        } else {
             let macro_args: Vec<proc_macro2::TokenStream> = service.metadata.generics.params.iter().map(|p| match p {
                syn::GenericParam::Type(t) => {
                    let ident = &t.ident;
                    quote! { $#ident:ident }
                },
                syn::GenericParam::Const(c) => {
                    let ident = &c.ident;
                    quote! { $#ident:expr }
                },
                syn::GenericParam::Lifetime(l) => {
                     let ident = &l.lifetime.ident;
                     quote! { $#ident:lifetime }
                }
            }).collect();
            (
                quote! { {$callback:path, $args:tt, $context:tt, $rest:tt, $seen:tt, (#(#macro_args),*)} },
                quote! { }
            )
        }
    } else if generic_params.is_empty() {
        (
            quote! { {$callback:path, $args:tt, $context:tt, $rest:tt, $seen:tt, $extra:tt} },
            quote! {
                #(
                    $callback! { $args, (dyn #transformed_interfaces), #impl_type, $context, $rest, $seen, $extra }
                )*
            }
        )
    } else {
        let macro_args: Vec<proc_macro2::TokenStream> = service.metadata.generics.params.iter().map(|p| match p {
            syn::GenericParam::Type(t) => {
                let ident = &t.ident;
                quote! { $#ident:ident }
            },
            syn::GenericParam::Const(c) => {
                let ident = &c.ident;
                quote! { $#ident:expr }
            },
            syn::GenericParam::Lifetime(l) => {
                 let ident = &l.lifetime.ident;
                 quote! { $#ident:lifetime }
            }
        }).collect();
        let macro_args_tokens = quote! { (#(#macro_args),*) };
        (
            quote! { {$callback:path, $args:tt, $context:tt, $rest:tt, $seen:tt, #macro_args_tokens} },
            quote! {
                #(
                    $callback! { $args, (dyn #transformed_interfaces), #impl_type, $context, $rest, $seen, #macro_args_tokens }
                )*
            }
        )
    };
    let linkage_macro = quote! {
        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! #macro_name_internal {
            #macro_pattern => {
                #macro_body
            };
        }
        #[allow(unused_imports)]
        pub use #macro_name_internal as #macro_name;
    };

    if component_name.to_string() == "RegisterServiceImpl" {
        eprintln!("Linkage macro for RegisterServiceImpl: {}", linkage_macro);
    }

    let interface_type = if interfaces.len() > 1 {
        quote! { Self }
    } else {
        quote! { dyn #interface }
    };

    // DefaultInterface implementation
    let default_interface_impl = quote! {
        impl #generic_impls_params ::shaku::DefaultInterface for #component_name #generic_tys #generic_where {
            type Interface = #interface_type;
        }
    };

    let register_interfaces_body = if interfaces.len() > 1 {
        quote! {
            #(
                context.register_interface::<dyn #interfaces>(
                    ::std::sync::Arc::clone(&component) as ::std::sync::Arc<dyn #interfaces>
                );
            )*
        }
    } else {
        quote! {}
    };

    let component_impl = quote! {
        impl<
            M: ::shaku::Module #(+ #dependencies)*,
            #generic_impls_no_parens
        > ::shaku::Component<M> for #component_name #generic_tys #generic_where {
            type Interface = #interface_type;
            type Parameters = #parameters_name #generic_tys;

            fn build(context: &mut ::shaku::ModuleBuildContext<M>, params: Self::Parameters) -> Box<Self::Interface> {
                Box::new(Self {
                    #(#resolve_properties),*
                })
            }

            fn register_interfaces(context: &mut ::shaku::ModuleBuildContext<M>, component: ::std::sync::Arc<Self::Interface>) {
                #register_interfaces_body
            }
        }

        #[doc = #parameters_doc]
        #visibility struct #parameters_name #generic_impls_params #generic_where {
            #(#parameters_properties),*
        }

        impl #generic_impls_params ::std::default::Default for #parameters_name #generic_tys #generic_where {
            #[allow(unreachable_code)]
            fn default() -> Self {
                Self {
                    #(#parameters_defaults),*
                }
            }
        }
    };

    let output = quote! {
        #linkage_macro
        #component_impl
        #default_interface_impl
    };

    // panic!("Derive output: {}", output);

    if debug_level > 0 {
        println!("{}", output);
    }

    Ok(output)
}

fn create_resolve_property(property: &Property) -> TokenStream {
    let property_name = &property.property_name;

    match property.property_type {
        PropertyType::Component | PropertyType::Provided => quote! {
            #property_name: M::build_component(context)
        },
        PropertyType::MultipleComponents => quote! {
            #property_name: M::collect(context)
        },
        _ => quote! {
            #property_name: params.#property_name
        },
    }
}

fn create_parameters_property(property: &Property, vis: &Visibility) -> Option<TokenStream> {
    if property.is_service() {
        return None;
    }

    let property_name = &property.property_name;
    let property_ty = &property.ty;
    let property_type = quote! { #property_ty };
    let doc_comment = &property.doc_comment;

    Some(quote! {
        #(#doc_comment)*
        #vis #property_name: #property_type
    })
}

fn create_parameters_default(property: &Property, component_ident: &Ident) -> Option<TokenStream> {
    if property.is_service() {
        return None;
    }
    let property_name = &property.property_name;
    match property.property_type {
        PropertyType::MultipleComponents => Some(quote! {
            #property_name: vec![]
        }),
        _ => match &property.default {
            PropertyDefault::Provided(default_expr) => Some(quote! {
                #property_name: #default_expr
            }),
            PropertyDefault::NotProvided => Some(quote! {
                #property_name: Default::default()
            }),
            PropertyDefault::NoDefault => {
                let unreachable_msg = format!(
                    "There is no default value for `{}::{}`",
                    component_ident, property_name
                );

                Some(quote! {
                    #property_name: unreachable!(#unreachable_msg)
                })
            }
        },
    }
}

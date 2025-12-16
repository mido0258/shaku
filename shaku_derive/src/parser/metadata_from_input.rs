use crate::consts;
use crate::parser::Parser;
use crate::structures::service::MetaData;
use syn::spanned::Spanned;
use syn::{DeriveInput, Error, Type};

use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitBool, Token};

enum ShakuMeta {
    Interface(Type),
    NoResolve(LitBool),
}

impl Parse for ShakuMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        let _: Token![=] = input.parse()?;
        if key == consts::INTERFACE_ATTR_NAME {
            let val: Type = input.parse()?;
            Ok(ShakuMeta::Interface(val))
        } else if key == "no_resolve" {
            let val: LitBool = input.parse()?;
            Ok(ShakuMeta::NoResolve(val))
        } else {
            Err(Error::new(key.span(), format!("Unknown key '{}'", key)))
        }
    }
}

impl Parser<MetaData> for DeriveInput {
    fn parse_as(&self) -> syn::Result<MetaData> {
        let mut interfaces = Vec::new();
        let mut no_resolve = false;
        let mut found_shaku_attr = false;

        for attr in &self.attrs {
            if !attr.path.is_ident(consts::ATTR_NAME) {
                continue;
            }
            found_shaku_attr = true;

            let metas = attr.parse_args_with(
                syn::punctuated::Punctuated::<ShakuMeta, syn::Token![,]>::parse_terminated
            ).map_err(|_| {
                Error::new(
                    attr.span(),
                    format!(
                        "Invalid attribute format. Example: #[{}({} = <your trait>, no_resolve = true)]",
                        consts::ATTR_NAME,
                        consts::INTERFACE_ATTR_NAME
                    ),
                )
            })?;

            for meta in metas {
                match meta {
                    ShakuMeta::Interface(ty) => interfaces.push(ty),
                    ShakuMeta::NoResolve(lit) => no_resolve = lit.value,
                }
            }
        }

        if !found_shaku_attr {
             return Err(Error::new(
                self.ident.span(),
                format!(
                    "Unable to find interface. Please add a '#[{}({} = <your trait>)]'",
                    consts::ATTR_NAME,
                    consts::INTERFACE_ATTR_NAME
                ),
            ));
        }

        if interfaces.is_empty() {
             return Err(Error::new(
                self.ident.span(),
                format!(
                    "Unable to find interface. Please add a '#[{}({} = <your trait>)]'",
                    consts::ATTR_NAME,
                    consts::INTERFACE_ATTR_NAME
                ),
            ));
        }

        Ok(MetaData {
            identifier: self.ident.clone(),
            generics: self.generics.clone(),
            interfaces,
            visibility: self.vis.clone(),
            no_resolve,
        })
    }
}

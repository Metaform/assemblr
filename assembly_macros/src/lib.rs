//  Copyright (c) 2026 Metaform Systems, Inc
//
//  This program and the accompanying materials are made available under the
//  terms of the Apache License, Version 2.0 which is available at
//  https://www.apache.org/licenses/LICENSE-2.0
//
//  SPDX-License-Identifier: Apache-2.0
//
//  Contributors:
//       Metaform Systems, Inc. - initial API and implementation
//

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Token, Type};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;

struct ServiceAssemblyArgs {
    name: Option<String>,
    provides: Vec<Type>,
    requires: Vec<Type>,
}

impl Parse for ServiceAssemblyArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name: Option<String> = None;
        let mut provides: Vec<Type> = Vec::new();
        let mut requires: Vec<Type> = Vec::new();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if ident == "name" {
                let lit: syn::LitStr = input.parse()?;
                name = Some(lit.value());
            } else if ident == "provides" {
                let content;
                syn::bracketed!(content in input);
                let types: Punctuated<Type, Token![,]> =
                    content.parse_terminated(Type::parse, Token![,])?;
                provides = types.into_iter().collect();
            } else if ident == "requires" {
                let content;
                syn::bracketed!(content in input);
                let types: Punctuated<Type, Token![,]> =
                    content.parse_terminated(Type::parse, Token![,])?;
                requires = types.into_iter().collect();
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ServiceAssemblyArgs {
            name,
            provides,
            requires,
        })
    }
}

#[proc_macro_attribute]
pub fn assembly(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    let args = parse_macro_input!(attr as ServiceAssemblyArgs);

    let assembly_name = args.name.unwrap_or_else(|| struct_name.to_string());
    let provides_types = args.provides;
    let requires_types = args.requires;

    // Generate the provides() method
    let provides_impl = if provides_types.is_empty() {
        quote! {
            fn provides(&self) -> Vec<TypeKey> {
                Vec::new()
            }
        }
    } else {
        quote! {
            fn provides(&self) -> Vec<TypeKey> {
                vec![#(TypeKey::new::<#provides_types>()),*]
            }
        }
    };

    // Generate the requires() method
    let requires_impl = if requires_types.is_empty() {
        quote! {
            fn requires(&self) -> Vec<TypeKey> {
                Vec::new()
            }
        }
    } else {
        quote! {
            fn requires(&self) -> Vec<TypeKey> {
                vec![#(TypeKey::new::<#requires_types>()),*]
            }
        }
    };

    // Generate the output
    let expanded = quote! {
        #input

        impl ServiceAssemblyBase for #struct_name {
            fn name(&self) -> &str {
                #assembly_name
            }

            #provides_impl

            #requires_impl
        }
    };

    TokenStream::from(expanded)
}
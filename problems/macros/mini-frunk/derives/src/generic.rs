#![forbid(unsafe_code)]
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn impl_generic(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident: ty_name,
        generics,
        data,
        ..
    } = parse_macro_input!(input);
    let Data::Struct(strct) = data else {
        panic!("underlying data should be struct")
    };

    let (fields_names, fields_types) = match strct.fields {
        Fields::Named(fields) => {
            let names = fields
                .named
                .iter()
                .map(|f| {
                    let name = f.ident.as_ref().unwrap();
                    quote! { #name }
                })
                .collect::<Vec<_>>();
            let types = fields
                .named
                .iter()
                .map(|f| {
                    let ty = &f.ty;
                    quote! { #ty }
                })
                .collect::<Vec<_>>();
            (names, types)
        }
        _ => panic!("only named fields are accepted."),
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote!(
        impl #impl_generics ::mini_frunk_core::generic::Generic for #ty_name #ty_generics #where_clause {
            type Repr = ::mini_frunk_core::HList![#(#fields_types),*];

            fn into(self) -> Self::Repr {
                let Self { #(#fields_names),*} = self;
                ::mini_frunk_core::hlist![#(#fields_names),*]
            }

            fn from(repr: Self::Repr) -> Self {
                let ::mini_frunk_core::hlist_pat![#(#fields_names),*] = repr;
                Self { #(#fields_names),* }
            }
        }
    )
    .into()
}

// TODO: your code goes here.

#![forbid(unsafe_code)]
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, Ident};

pub fn impl_labelled(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident: ty_name,
        generics,
        data,
        ..
    } = parse_macro_input!(input);
    let Data::Struct(strct) = data else {
        panic!("underlying data should be struct")
    };

    let (field_names, field_types, field_names_enum) = match strct.fields {
        Fields::Named(fields) => {
            let field_name_enum = fields
                .named
                .iter()
                .map(|f| {
                    let field_name = f.ident.as_ref().unwrap().to_string();
                    let stringified = field_name
                        .chars()
                        .into_iter()
                        .map(|ch| {
                            if ch == '_' {
                                Ident::new("__", f.span())
                            } else {
                                Ident::new(&ch.to_string(), f.span())
                            }
                        })
                        .map(|ch_ident| quote!( ::mini_frunk_core::field::symbols::#ch_ident));
                    quote!((#(#stringified), *))
                })
                .collect::<Vec<_>>();

            let field_ty = fields.named.iter().map(|f| {
                let field_ty = f.ty.clone();
                quote!(#field_ty)
            });

            let field_name_ty = field_name_enum
                .iter()
                .zip(field_ty)
                .map(|(name, ty)| quote!( ::mini_frunk_core::field::Field<#name, #ty>))
                .collect::<Vec<_>>();

            let names = fields
                .named
                .iter()
                .map(|f| {
                    let name = f.ident.clone().unwrap();
                    quote! { #name }
                })
                .collect::<Vec<_>>();

            (names, field_name_ty, field_name_enum)
        }
        _ => panic!("only named fields are accepted."),
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote!(
        impl #impl_generics ::mini_frunk_core::labelled::LabelledGeneric for #ty_name #ty_generics #where_clause {
            type Repr = ::mini_frunk_core::HList![#(#field_types),*];

            fn into(self) -> Self::Repr {
                let Self { #(#field_names),*} = self;
                ::mini_frunk_core::hlist![#(::mini_frunk_core::field!(#field_names_enum, #field_names)),*]
            }

            fn from(repr: Self::Repr) -> Self {
                let ::mini_frunk_core::hlist_pat![#(#field_names),*] = repr;
                Self { #(#field_names: #field_names.value),* }
            }
        }
    )
    .into()
}

// TODO: your code goes here.

#![forbid(unsafe_code)]
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, FieldsNamed};

#[proc_macro_derive(Scan)]
pub fn derive_scan(input: TokenStream) -> TokenStream {
    // for a given struct collect all named fields
    // and try to get references to them
    let DeriveInput {
        ident: ty_name,
        generics,
        data,
        ..
    } = parse_macro_input!(input);
    let Data::Struct(strct) = data else {
        panic!("underlying data should be struct")
    };

    let fields_quotes = match strct.fields {
        Fields::Named(FieldsNamed { named: fields, .. }) => fields
            .into_iter()
            .map(|Field { ident: f_ident, .. }| {
                quote!(
                    ret_gcs.extend((&self.#f_ident as &dyn Scan).get_children_ref_adrrs());
                )
            })
            .collect(),
        _ => vec![],
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote!(
        impl #impl_generics Scan for #ty_name #ty_generics  #where_clause
        {
            fn get_children_ref_adrrs(&self) -> Vec<usize>{
                let mut ret_gcs = Vec::new();
                #(#fields_quotes)*
                ret_gcs
            }
        }

    )
    .into()
}

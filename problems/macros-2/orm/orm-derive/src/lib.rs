#![forbid(unsafe_code)]
use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, LitStr};

#[proc_macro_derive(Object, attributes(table_name, column_name))]
pub fn derive_object(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident: ty_name,
        attrs,
        data,
        generics,
        ..
    } = parse_macro_input!(input);

    let Data::Struct(strct) = data else {
        panic!("underlying data should be struct")
    };

    // table name attr
    let table_name = get_str_attr_val("table_name", &attrs).unwrap_or_else(|| ty_name.to_string());

    let (fields_names, fields_types, fields_columns) = match strct.fields {
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
            let columns = fields
                .named
                .iter()
                .map(|f| {
                    get_str_attr_val("column_name", &f.attrs)
                        .unwrap_or_else(|| f.ident.as_ref().unwrap().to_string())
                })
                .collect::<Vec<_>>();
            (names, types, columns)
        }
        _ => (vec![], vec![], vec![]),
    };

    let obj_fields = fields_names
        .iter()
        .zip(fields_columns.iter())
        .zip(fields_types.iter())
        .map(|((ident, column), ty)| {
            quote!(
                ::orm::object::ObjectField {
                    name: stringify!(#ident),
                    column: #column,
                    data_ty: <#ty as ::orm::data::DataTypeWrapper>::TYPE,
                }
            )
        })
        .collect::<Vec<_>>();
    let to_row = fields_names
        .iter()
        .map(|f_name| quote!((&self.#f_name).into()))
        .collect::<Vec<_>>();

    let from_row = fields_names
        .iter()
        .map(|f_name| quote!(#f_name: r_iter.next().unwrap().into()))
        .collect::<Vec<_>>();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let resulting_gen = quote!(
        impl #impl_generics Object for #ty_name #ty_generics  #where_clause
        {
            fn to_row(&self) -> ::orm::storage::Row {
                vec![#(#to_row),*]
            }

            fn from_row(row: ::orm::storage::Row) -> Self {
                let mut r_iter = row.into_iter();
                Self {#(#from_row),*}
            }

            fn get_schema() -> &'static ::orm::object::Schema {
                &::orm::object::Schema {
                    obj_ty: stringify!(#ty_name),
                    table_name: #table_name,
                    obj_fields: &[#(#obj_fields),*]
                }
            }
        }
    );
    resulting_gen.into()
}

fn get_str_attr_val(attr_name: &str, attrs: &Vec<Attribute>) -> Option<String> {
    attrs
        .iter()
        .find(|keyword| keyword.path().is_ident(attr_name))
        .and_then(|attr| attr.parse_args::<LitStr>().ok().map(|ls| ls.value()))
}

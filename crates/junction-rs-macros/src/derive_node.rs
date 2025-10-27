use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit};

/// Derive `junction_rs::traits::Node` for a struct and generate a `Schema` module with typed columns.
pub(crate) fn derive_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();
    let mut table_name: Option<String> = None;
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("junction")) {
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("table") {
                let ts: proc_macro2::TokenStream = meta.value()?.parse()?;
                if let Ok(Lit::Str(ls)) = syn::parse2::<Lit>(ts.clone()) {
                    table_name = Some(ls.value());
                }
                if table_name.is_none() {
                    if let Ok(path) = syn::parse2::<syn::Path>(ts) {
                        if let Some(seg) = path.segments.last() {
                            table_name = Some(seg.ident.to_string());
                        }
                    }
                }
            }
            Ok(())
        });
    }
    let table_name = table_name.unwrap_or_else(|| {
        let s = name.to_string();
        let mut out = String::with_capacity(s.len());
        let mut prev_is_lower_or_digit = false;
        for ch in s.chars() {
            if ch.is_ascii_uppercase() {
                if !out.is_empty() && prev_is_lower_or_digit {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
                prev_is_lower_or_digit = false;
            } else {
                out.push(ch);
                prev_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            }
        }
        out
    });
    let mod_ident = format_ident!("{}", table_name);
    let mut schema_field_decls = Vec::new();
    let mut schema_field_inits = Vec::new();
    let mut id_field_ty: Option<proc_macro2::TokenStream> = None;
    if let Data::Struct(st) = &input.data {
        if let Fields::Named(fields) = &st.fields {
            for f in fields.named.iter() {
                let ident = f.ident.as_ref().unwrap();
                let ty = &f.ty;
                let col_name = ident.to_string();
                if col_name.trim_start_matches("r#") == "id" {
                    id_field_ty = Some(quote! { #ty });
                    continue;
                }
                schema_field_decls.push(quote! { pub #ident: junction_rs::schema::Col<#ty> });
                schema_field_inits.push(quote! { #ident: junction_rs::schema::Col::new(<#name as junction_rs::traits::Node>::TABLE, #col_name) });
            }
        }
    }
    let id_ty = id_field_ty.unwrap_or_else(|| quote! { junction_rs::id::SimpleId<#name> });
    let mut final_schema_field_decls = Vec::with_capacity(schema_field_decls.len() + 1);
    let mut final_schema_field_inits = Vec::with_capacity(schema_field_inits.len() + 1);
    final_schema_field_decls.push(quote! { pub id: junction_rs::schema::Col<#id_ty> });
    final_schema_field_inits.push(quote! { id: junction_rs::schema::Col::new(<#name as junction_rs::traits::Node>::TABLE, "id") });
    final_schema_field_decls.extend(schema_field_decls);
    final_schema_field_inits.extend(schema_field_inits);
    schema_field_decls = final_schema_field_decls;
    schema_field_inits = final_schema_field_inits;
    let expanded = quote! {
      impl junction_rs::traits::Node for #name {
          const TABLE: &'static str = #table_name;
          type Schema = #mod_ident::Schema;
          fn schema() -> Self::Schema { #mod_ident::Schema::new() }
      }
      impl junction_rs::traits::Insertable for #name {
          fn table_name() -> &'static str { <#name as junction_rs::traits::Node>::TABLE }
      }
      impl junction_rs::traits::HasTableName for #name { fn table_name() -> &'static str { <#name as junction_rs::traits::Node>::TABLE } }
      impl #name {
          /// Create a new randomly generated `SimpleId<#name>`.
          pub fn create_simple_id() -> junction_rs::id::SimpleId<#name> {
              // Uses WithId blanket impl; kept as inherent for discoverability.
              <#name as junction_rs::traits::WithId>::create_simple_id()
          }
          /// Wrap an existing `uuid::Uuid` into a strongly typed `SimpleId<#name>`.
          pub fn from_uuid(id: uuid::Uuid) -> junction_rs::id::SimpleId<#name> {
              <#name as junction_rs::traits::WithId>::from_uuid(id)
          }
      }
      pub mod #mod_ident {
          use super::*;
          pub struct Schema { #( #schema_field_decls, )* }
          impl Schema {
              pub fn new() -> Self {
                  Self { #( #schema_field_inits, )* }
              }
          }
      }
    };
    TokenStream::from(expanded)
}

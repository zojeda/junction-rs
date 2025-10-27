use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit};

/// Derive `junction_rs::traits::Edge` for a struct and generate a `Schema` module with typed columns.
pub(crate) fn derive_edge(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();
    let mut table_name: Option<String> = None;
    // Parse #[junction(...)] attributes
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("junction")) {
        if let Err(err) = attr.parse_nested_meta(|meta| {
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
                Ok(())
            } else if meta.path.is_ident("from") || meta.path.is_ident("to") {
                Err(meta.error("#[junction(from = ...)] and #[junction(to = ...)] are no longer supported; annotate fields with #[junction(source)] / #[junction(target)] instead"))
            } else {
                Err(meta.error("unsupported option inside #[junction(...)]"))
            }
        }) {
            return err.to_compile_error().into();
        }
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
    let mut id_field_ty: Option<proc_macro2::TokenStream> = None;
    let mut source_endpoint_ty: Option<proc_macro2::TokenStream> = None;
    let mut target_endpoint_ty: Option<proc_macro2::TokenStream> = None;
    let mut schema_field_decls = Vec::new();
    let mut schema_field_inits = Vec::new();
    // Track original user field names (normalized) for source/target endpoints
    let mut source_field_name: Option<String> = None;
    let mut target_field_name: Option<String> = None;

    fn trim_raw_ident(ident: &syn::Ident) -> String {
        ident.to_string().trim_start_matches("r#").to_string()
    }

    fn extract_endpoint_node_type(ty: &syn::Type) -> Option<proc_macro2::TokenStream> {
        use syn::{GenericArgument, PathArguments, Type};

        fn inner_from_type(ty: &Type) -> Option<&Type> {
            match ty {
                Type::Path(tp) => {
                    let segment = tp.path.segments.last()?;
                    let ident = segment.ident.to_string();
                    match ident.as_str() {
                        "SimpleId" | "Link" => {
                            if let PathArguments::AngleBracketed(ab) = &segment.arguments {
                                for arg in &ab.args {
                                    if let GenericArgument::Type(inner) = arg {
                                        return Some(inner);
                                    }
                                }
                            }
                            None
                        }
                        "Option" | "Vec" => {
                            if let PathArguments::AngleBracketed(ab) = &segment.arguments {
                                for arg in &ab.args {
                                    if let GenericArgument::Type(inner) = arg {
                                        if let Some(found) = inner_from_type(inner) {
                                            return Some(found);
                                        }
                                    }
                                }
                            }
                            None
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }

        inner_from_type(ty).map(|inner| quote! { #inner })
    }

    // Walk fields to build schema and perform validation
    if let Data::Struct(st) = &input.data {
        if let Fields::Named(fields) = &st.fields {
            for f in fields.named.iter() {
                let ident = f.ident.as_ref().unwrap();
                let ty = &f.ty;
                let mut role = None;
                for attr in f.attrs.iter().filter(|a| a.path().is_ident("junction")) {
                    if let Err(err) = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("source") {
                            if role.replace("source").is_some() {
                                return Err(
                                    meta.error("duplicate #[junction(source)] on the same field")
                                );
                            }
                            Ok(())
                        } else if meta.path.is_ident("target") {
                            if role.replace("target").is_some() {
                                return Err(
                                    meta.error("duplicate #[junction(target)] on the same field")
                                );
                            }
                            Ok(())
                        } else {
                            Err(meta.error("unsupported option inside #[junction(...)] for fields"))
                        }
                    }) {
                        return err.to_compile_error().into();
                    }
                }

                let ident_norm = trim_raw_ident(ident);
                if ident_norm == "id" {
                    id_field_ty = Some(quote! { #ty });
                } else {
                    let col_name = ident_norm.clone();
                    schema_field_decls.push(quote! { pub #ident: junction_rs::schema::Col<#ty> });
                    schema_field_inits.push(quote! { #ident: junction_rs::schema::Col::new(<#name as junction_rs::traits::Edge>::TABLE, #col_name) });
                }

                match role {
                    Some("source") => {
                        if source_endpoint_ty.is_some() {
                            return quote! { compile_error!("Only one field may be annotated with #[junction(source)]"); }.into();
                        }
                        if let Some(node_ty) = extract_endpoint_node_type(ty) {
                            source_endpoint_ty = Some(node_ty);
                            source_field_name = Some(ident_norm.clone());
                        } else {
                            return quote! { compile_error!("Could not infer source node type from field annotated with #[junction(source)]"); }.into();
                        }
                    }
                    Some("target") => {
                        if target_endpoint_ty.is_some() {
                            return quote! { compile_error!("Only one field may be annotated with #[junction(target)]"); }.into();
                        }
                        if let Some(node_ty) = extract_endpoint_node_type(ty) {
                            target_endpoint_ty = Some(node_ty);
                            target_field_name = Some(ident_norm.clone());
                        } else {
                            return quote! { compile_error!("Could not infer target node type from field annotated with #[junction(target)]"); }.into();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    if source_endpoint_ty.is_none() {
        return quote! { compile_error!("Missing #[junction(source)] field on Edge struct"); }
            .into();
    }
    if target_endpoint_ty.is_none() {
        return quote! { compile_error!("Missing #[junction(target)] field on Edge struct"); }
            .into();
    }

    let id_ty = id_field_ty.unwrap_or_else(|| quote! { junction_rs::id::SimpleId<#name> });
    let mut final_schema_field_decls = Vec::with_capacity(schema_field_decls.len() + 1);
    let mut final_schema_field_inits = Vec::with_capacity(schema_field_inits.len() + 1);

    final_schema_field_decls.push(quote! { pub id: junction_rs::schema::Col<#id_ty> });
    final_schema_field_inits.push(quote! { id: junction_rs::schema::Col::new(<#name as junction_rs::traits::Edge>::TABLE, "id") });
    final_schema_field_decls.extend(schema_field_decls);
    final_schema_field_inits.extend(schema_field_inits);
    schema_field_decls = final_schema_field_decls;
    schema_field_inits = final_schema_field_inits;

    let mod_ident = format_ident!("{}", table_name);
    let from_ty = source_endpoint_ty.unwrap();
    let to_ty = target_endpoint_ty.unwrap();
    let src_field_const = source_field_name.expect("source_field_name set");
    let tgt_field_const = target_field_name.expect("target_field_name set");
    let expanded = quote! {
        impl junction_rs::traits::Edge for #name {
            const TABLE: &'static str = #table_name;
            type Schema = #mod_ident::Schema;
            type From = #from_ty;
            type To = #to_ty;
            const SOURCE_FIELD_NAME: &'static str = #src_field_const;
            const TARGET_FIELD_NAME: &'static str = #tgt_field_const;
            fn schema() -> Self::Schema { #mod_ident::Schema::new() }
        }
        impl junction_rs::traits::Insertable for #name {
            fn table_name() -> &'static str { <#name as junction_rs::traits::Edge>::TABLE }
            fn endpoint_fields() -> Option<(&'static str, &'static str)> { Some((Self::SOURCE_FIELD_NAME, Self::TARGET_FIELD_NAME)) }
        }
        impl junction_rs::traits::HasTableName for #name { fn table_name() -> &'static str { <#name as junction_rs::traits::Edge>::TABLE } }
        impl #name {
            pub const TABLE: &'static str = <#name as junction_rs::traits::Edge>::TABLE;
            /// Name of the struct field annotated with #[junction(source)] (normalized: raw identifiers stripped of r# prefix).
            pub const SOURCE_FIELD_NAME: &'static str = #src_field_const;
            /// Name of the struct field annotated with #[junction(target)].
            pub const TARGET_FIELD_NAME: &'static str = #tgt_field_const;
            pub fn schema() -> <#name as junction_rs::traits::Edge>::Schema {
                <#name as junction_rs::traits::Edge>::schema()
            }
            pub fn create_simple_id() -> junction_rs::id::SimpleId<#name> {
                <#name as junction_rs::traits::WithId>::create_simple_id()
            }
            pub fn from_uuid(id: uuid::Uuid) -> junction_rs::id::SimpleId<#name> {
                <#name as junction_rs::traits::WithId>::from_uuid(id)
            }
            /// Convenience helper returning (source_field_name, target_field_name)
            pub fn endpoint_field_names() -> (&'static str, &'static str) {
                (Self::SOURCE_FIELD_NAME, Self::TARGET_FIELD_NAME)
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

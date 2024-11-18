use itertools::Itertools;
use ormlite_attr::Ident;
use ormlite_attr::ModelMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn struct_InsertModel(ast: &DeriveInput, attr: &ModelMeta) -> TokenStream {
    let Some(insert_model) = &attr.insert_struct else {
        return quote! {};
    };

    // Get the fields from the original struct
    let fields = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => return quote! {},
        },
        _ => return quote! {},
    };

    let vis = &ast.vis;
    let struct_fields = attr.columns.iter()
        .filter(|c| !c.is_default())
        .map(|c| {
            let id = &c.ident;
            let ty = &c.ty;
            
            // Find the original field to get its attributes
            let original_field = fields.iter().find(|f| {
                f.ident.as_ref()
                    .map(|i| i.to_string() == id.to_string())
                    .unwrap_or(false)
            });
            
            // Get the original attributes, excluding the ormlite attribute
            let attrs = original_field.map(|f| {
                f.attrs.iter()
                    .filter(|a| {
                        // Keep all attributes except ormlite
                        !a.path().is_ident("ormlite")
                    })
                    .collect::<Vec<_>>()
            }).unwrap_or_default();
            
            quote! {
                #(#attrs)*
                pub #id: #ty
            }
        });

    if let Some(extra_derives) = &attr.extra_derives {
        quote! {
            #[derive(Debug, #(#extra_derives,)*)]
            #vis struct #insert_model {
                #(#struct_fields,)*
            }
        }    
    } else {
        quote! {
            #[derive(Debug)]
            #vis struct #insert_model {
                #(#struct_fields,)*
            }
        }
    }
}

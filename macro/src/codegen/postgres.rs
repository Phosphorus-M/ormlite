use crate::codegen::common::OrmliteCodegen;
use ormlite_attr::TableMetadata;
use ormlite_core::query_builder::Placeholder;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub struct PostgresBackend {}

impl OrmliteCodegen for PostgresBackend {
    fn database(&self) -> TokenStream {
        quote! { ::ormlite::postgres::Postgres }
    }

    fn placeholder(&self) -> TokenStream {
        quote! {
            ::ormlite::query_builder::Placeholder::dollar_sign()
        }
    }

    fn raw_placeholder(&self) -> Placeholder {
        Placeholder::dollar_sign()
    }

    fn impl_Model__select(&self, _ast: &DeriveInput, attr: &TableMetadata) -> TokenStream {
        let table_name = &attr.table_name;
        let db = self.database();
        quote! {
            fn select<'args>() -> ::ormlite::query_builder::SelectQueryBuilder<'args, #db, Self> {
                ::ormlite::query_builder::SelectQueryBuilder::default()
                    .select(&format!("\"{}\".*", #table_name))
            }
        }
    }
}

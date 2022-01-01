#![cfg_attr(feature = "insert", feature(inherent_associated_types))]

pub use ormlite_core::BoxFuture;
pub use ormlite_core::SelectQueryBuilder;
pub use ormlite_core::{Error, Result};
pub use ormlite_macro::Model;
pub use sqlx::sqlx_macros::FromRow;

pub mod export;
pub mod model;

#[cfg(feature = "handwritten")]
pub mod handwritten;

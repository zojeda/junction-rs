// Reexport everything from junction-rs-core and junction-rs-macros
//! # junction-rs
//!
//! Unified ORM crate for Rust, reexporting everything from `junction-rs-core` and `junction-rs-macros`.
//!
//! ## Example: Using Derive Macros
//!
//! ```rust
//! mod example {
//!   use junction_rs::prelude::*;
//!
//!   #[derive(Debug, Clone, Serialize, Deserialize, Node)]
//!   #[junction(table = "user")]
//!   struct User {
//!       id: SimpleId<User>,
//!       name: String,
//!   }
//!
//!   #[derive(Debug, Clone, Serialize, Deserialize, Edge)]
//!   #[junction(table = "friendship")]
//!   struct Friendship {
//!       id: SimpleId<Friendship>,
//!       #[junction(source)]
//!       r#in: SimpleId<User>,
//!       #[junction(target)]
//!       r#out: SimpleId<User>,
//!   }
//! }
//! ```
//!
//! ## Prelude
//! Use all ORM features with:
//! ```rust
//! use junction_rs::prelude::*;
//! ```
//!
//! The prelude now also exports `IntoSelectColumn` letting you pass schema columns
//! directly to projection helpers: `select::<User>(All).from(User::table()).column(User::schema().name)`.
pub use junction_rs_core::*;
pub use junction_rs_macros::*;
#[cfg(feature = "surreal")]
pub use junction_rs_surrealdb::*;

/// Prelude module for convenient imports
pub mod prelude {
    pub use junction_rs_core::id::SimpleId;
    pub use junction_rs_core::prelude::*;
    pub use junction_rs_core::traits::*;
    pub use junction_rs_core::traits::WithId;
    pub use junction_rs_macros::*;
    #[cfg(feature = "surreal")]
    pub use junction_rs_surrealdb::*;
    pub use serde::{Deserialize, Serialize};
    pub use junction_rs_core::get_by_id;
}

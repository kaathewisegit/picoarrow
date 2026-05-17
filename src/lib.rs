#![cfg_attr(docsrs, feature(doc_cfg))]

//! A tiny package which implements a subset of [Apache Arrow][aa]
//! functionality.
//!
//! [aa]: https://arrow.apache.org/

pub mod array;
pub(crate) mod bitmap;
mod error;
pub(crate) mod fb;
pub mod ipc;
mod schema;

pub use error::{Error, Result};
pub use schema::{Field, Schema};

mod seal {
	pub trait Seal {}
}

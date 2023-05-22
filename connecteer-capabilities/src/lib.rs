#![feature(
    // impl_trait_in_assoc_type,
    generators,
    generator_trait,
    type_alias_impl_trait,
)]
#![no_std]

//#![warn(clippy::pedantic)]

mod connection;
pub mod identity;
mod middleware;
mod pipeline;
mod sealed;

pub use connection::Connection;
pub use identity::Base;
pub use middleware::Middleware;
pub use pipeline::Pipeline;

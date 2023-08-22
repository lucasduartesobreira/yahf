#![allow(incomplete_features)]
#![feature(return_position_impl_trait_in_trait)]

/// YAHF is an web framework for Rust focused on developer experience, extensibility, and
/// performance.
pub mod deserializer;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod request;
pub mod response;
pub mod result;
pub mod router;
pub mod runner_input;
pub mod runner_output;
pub mod serializer;
pub mod server;
pub mod tree;

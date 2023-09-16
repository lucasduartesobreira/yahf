#![allow(incomplete_features)]
#![feature(return_position_impl_trait_in_trait)]
#![warn(missing_docs)]

//! YAHF is an web framework for Rust focused on developer experience, extensibility, and
//! performance.
//!
//! # Table of Contents
//! - [Features](#features)
//! - [Example](#example)
//! - [Routing](#routing)
//! - [Handlers](#handlers)
//! - [Extensability](#extensability)
//! - [Middleware](#middleware)
//! - [Examples](#examples)
//!
//! # Features
//!
//! - Macro free Routing API
//! - Predictable error handling
//! - Native serialization and deserialization built into the handler
//! - Friendly syntax
//!
//! # Example
//!
//! The `Hello world` of YAHF is:
//!
//! ```rust,no_run
//! use yahf::server::Server;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = Server::new().get(
//!         "/",
//!         || async { "Hello world".to_string() },
//!         &(),
//!         &String::with_capacity(0),
//!     );
//!
//!     server
//!         .listen(([127, 0, 0, 1], 8000).into())
//!         .await
//!         .unwrap();
//! }
//!
//! ```
//!
//! # Routing
//!
//! [`Router`](router::Router) is used to bind handlers to paths.\
//!
//! ```no_run
//! use yahf::router::Router;
//!
//! // Router
//! let router = Router::new()
//!     .get("/", root_get, &(), &())
//!     .get("/foo", foo_get, &(), &())
//!     .post("/foo", foo_post, &(), &())
//!     .delete("/foo/bar", bar_delete, &(), &());
//!
//! // calls respectively each of these handlers
//!
//! async fn root_get() {}
//! async fn foo_get() {}
//! async fn foo_post() {}
//! async fn bar_delete() {}
//!
//! # async {
//! #   yahf::server::Server::new().router(router);
//! # };
//! ```
//!
//! [`Server`](server::Server) shares these features from [`Router`](router::Router)
//!
//! # Handlers
//!
//! On YAHF, a `handler` is a async function that is used to handle a `Route`. An acceptable
//! `handler` implements the trait [`Runner`](handler::Runner). By default, these signatures are
//! supported:
//!
//! ```no_run
//! # use serde::Serialize;
//! # use serde::Deserialize;
//! # use yahf::result::Result;
//! use yahf::request::Request;
//! use yahf::response::Response;
//! # #[derive(Serialize, Deserialize)]
//! # struct ResponseBody { second_value: u64 };
//! # #[derive(Serialize, Deserialize)]
//! # struct RequestBody { first_value: u64 }
//!
//! async fn handler1() -> ResponseBody  
//! # {todo!()}
//! async fn handler2() -> Response<ResponseBody>  
//! # {todo!()}
//! async fn handler3(req: RequestBody) -> ResponseBody  
//! # {todo!()}
//! async fn handler4(req: Request<RequestBody>) -> ResponseBody
//! # {todo!()}
//! async fn handler5(req: RequestBody) -> Response<ResponseBody>
//! # {todo!()}
//! async fn handler6(req: Request<RequestBody>) -> Response<ResponseBody>
//! # {todo!()}
//! async fn handler7() -> Result<ResponseBody>
//! # {todo!()}
//! async fn handler8() -> Result<Response<ResponseBody>>
//! # {todo!()}
//! async fn handler9(req: Result<RequestBody>) -> Result<ResponseBody>
//! # {todo!()}
//! async fn handler10(req: Result<Request<RequestBody>>) -> Result<ResponseBody>
//! # {todo!()}
//! async fn handler11(req: Result<RequestBody>) -> Result<Response<ResponseBody>>
//! # {todo!()}
//! async fn handler12(req: Result<Request<RequestBody>>) -> Result<Response<ResponseBody>>
//! # {todo!()}
//! ```
//!
//! All these signatures comes from the implementations of [`RunnerInput`](runner_input::RunnerInput) and [`RunnerOutput`](runner_output::RunnerOutput).
//!
//! # Extensability
//!
//! YAHF `handlers` are modular by design. A `handler` is decomposed into four modules: a body [`deserializer`](deserializer::BodyDeserializer),
//! a body [`serializer`](serializer::BodySerializer), [`arguments`](runner_input::RunnerInput), and a [`response`](runner_output::RunnerOutput).
//! These modules are glued together using the [`Runner`](handler::Runner) trait. Adding new
//! functionality to the handlers is just a matter of implementing one of these traits. For more
//! details, check the trait docs
//!
//! #### Deserialization/Serialization
//!
//! `Deserialization` and `Serialization` are applied only to the `Body` of a `Request`/`Response`.
//! These two operations are handled respectively by [`BodyDeserializer`](serializer::BodySerializer),
//! [`BodyDeserializer`](deserializer::BodyDeserializer). More details its docs.
//!
//! #### Function Arguments/Response
//!
//! # Middleware
//!
//!
//! # Examples
//!

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

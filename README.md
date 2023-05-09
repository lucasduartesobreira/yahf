# Yet Another HTTP Framework

> **Warning: Currently in the experimental phase, everything can change.**

The goal of `YAHF` is both to provide a good developer experience and to be easy to extend. 

Following the author's vision, those are the project rules:
It runs on stable rust;
Serialization and Deserialization internally dealt with;
No new macros;

## Goals for v1.0.0

> **`YAHF` follows the `SemVer`.**

The objective for v1.0.0 is to have a stable project that can deal with real-world problems with good developer experience and the possibility to extend the project to suit any need.

The goals for this version are:

- [ ] Composable routing system;
- [ ] Middleware functions;
- [ ] HTTP/1.1 with or without security.

### Code examples

The default code structure will look something like this

#### Handlers

Any function that follows some of these signatures will be considered an handler:

```rust
async fn handler_name(req: Request<ReqBodyType>) -> Response<ResBodyType> {/*Some code*/}
async fn handler_name(req: ReqBodyType) -> Response<ResBodyType> {/*Some code*/}
async fn handler_name() -> Response<ResBodyType> {/*Some code*/}
async fn handler_name(req: Request<ReqBodyType>) -> ResBodyType {/*Some code*/}
async fn handler_name(req: ReqBodyType) -> ResBodyType {/*Some code*/}
async fn handler_name() -> ResBodyType {/*Some code*/}
```

#### Routing

There will be two structures that will be used to setup routes, one is the `Server` and the other is the `Router`.
Both will follow the same pattern to register a new `route` in them, but only `Server` will be able to start listening for requests.

##### Adding Routes

Adding routes will be:

```rust
// Registering an handler for a route and a method
router.<method>("/path/goes/here", handler, RequestBodyDeserializer, ResponseBodySerializer);
```

Here both `Deserializer` and `Serializer` are structs with zero data. For more details look into the [#2]( https://github.com/lucasduartesobreira/yahf/pull/2 ).

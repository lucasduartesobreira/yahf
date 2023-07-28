
<p align="center">
  <img height="128" src="https://github.com/lucasduartesobreira/yahf/assets/58451227/ad1b8cb2-8f40-497f-83ab-1617179eb8cf" alt="YAHF">
</p>

<h1 align="center">
    <b>Yet Another HTTP Framework</b>
</h1>

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


##### Router

`Router` is the way to compose the structure, as it will allow to merge with other routers, this includes the `Server`. This is an example of what would be the usage of a `Router`:

```rust
let some_router: Router = Router::new();
some_router.all('/user', /*Some handler*/, &(), &Json);

let another_router: Router = Router::new();
another_router.all('/function', /*Some handler*/, &(), &String);

let updated_server: Server = server.router(some_router)?.router(another_router)?;
```

There is one more thing, `Router` as we'll see it later, will be the way to apply a middleware to an set of routes.


##### Server

`Server` is the core structure of an application as everything around is needed to set the `Server` up. Mainly, after setting up, running the application it's simple as:

```rust
server.listen('/*The IpAddress to bind and start listen*/')
```

#### Middleware

It'll be supported two types of middleware functions: `PreMiddleware` and `AfterMiddleware`

##### PreMiddleware

Apply transformations to `Request`:

```rust
async fn some_middleware(request: Result<Request<String>>) -> Result<Request<String>> { /*Function body*/ }


// When building a `Router` or a `Server`
let router = router.pre(some_middleware);
```


##### AfterMiddleware

Apply transformations to `Response`:

```rust
async fn some_after_middleware(response: Result<Response<String>>) -> Result<Response<String>> { /*Function body*/}


// When building a `Router` or a `Server`
let router = router.after(some_middleware);
```

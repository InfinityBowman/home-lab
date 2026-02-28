# Rust & Codebase Guide

A walkthrough of everything in the `crates/` directory, aimed at someone new to Rust.

---

## Table of Contents

- [Cargo & Workspaces](#cargo--workspaces)
- [The Crates](#the-crates)
  - [homelab-core](#homelab-core--the-vocabulary)
  - [homelab-db](#homelab-db--database-layer)
  - [homelab-docker](#homelab-docker--docker-integration)
  - [homelab-api](#homelab-api--http-server)
- [Rust Concepts Reference](#rust-concepts-reference)
- [Further Reading](#further-reading)

---

## Cargo & Workspaces

**Cargo** is Rust's build tool and package manager — like npm for Node or pip for Python. Every Rust project has a `Cargo.toml` that declares its name, version, and dependencies.

A **workspace** organizes a project into multiple smaller packages called **crates** that share dependencies. Our root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/homelab-core",
    "crates/homelab-db",
    "crates/homelab-docker",
    # ...
]
```

**Why split into crates instead of one big project?**

- **Faster builds** — Cargo only recompiles crates that changed
- **Clear boundaries** — the Docker code can't accidentally reach into database internals
- **Focused dependencies** — each crate lists only what it needs

The `[workspace.dependencies]` section is shared version pinning. Individual crates opt in with `serde = { workspace = true }` instead of repeating versions everywhere.

**Key commands:**

| Command | What it does |
|---------|-------------|
| `cargo build` | Compile the whole workspace |
| `cargo run -p homelab-api` | Run a specific crate's binary |
| `cargo check` | Type-check without building (faster) |
| `cargo test` | Run all tests |
| `cargo add <dep>` | Add a dependency to the current crate |

---

## The Crates

### homelab-core — The Vocabulary

**Location:** `crates/homelab-core/src/`

This defines the shared types every other crate uses. No business logic, just data shapes and error definitions.

#### Structs

Structs are like TypeScript interfaces or Python dataclasses — named groups of fields:

```rust
pub struct App {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub port: i64,
    pub status: AppStatus,
    // ...
}
```

- `pub` means other crates can access this field. Without it, the field is private.
- Every field must have an explicit type. Rust has no `any` type.

#### Enums

An enum is a value that must be one of several variants:

```rust
pub enum AppStatus {
    Created,
    Building,
    Running,
    Stopped,
    Failed,
}
```

Rust enums are more powerful than in most languages — each variant can hold data (like `NotFound(String)`). The compiler **forces** you to handle every variant when pattern matching, so you can't forget the `Failed` case.

#### Derive Macros

The `#[derive(...)]` annotations auto-generate code:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App { ... }
```

| Derive | What it generates |
|--------|------------------|
| `Debug` | Lets you print the struct with `{:?}` for debugging |
| `Clone` | Lets you duplicate the value (needed because of Rust's ownership rules — see below) |
| `Serialize` | Auto-converts the struct **to** JSON (from the `serde` library) |
| `Deserialize` | Auto-converts **from** JSON into the struct |

#### Error Type

```rust
#[derive(Debug, Error)]
pub enum HomelabError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("docker error: {0}")]
    Docker(String),
    // ...
}
```

`thiserror` is a library that implements Rust's standard `Error` trait. The `{0}` is a format placeholder for the contained `String`. So `HomelabError::NotFound("my-app".into())` displays as `"not found: my-app"`.

**Why a custom error type?** Rust doesn't have exceptions. Functions that can fail return `Result<T, E>` — either `Ok(value)` or `Err(error)`. One error enum covering all failure modes lets every crate use `Result<T, HomelabError>` consistently.

---

### homelab-db — Database Layer

**Location:** `crates/homelab-db/src/`

Uses **sqlx**, a Rust library for SQL databases with compile-time query checking.

#### Connection Pool

```rust
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}
```

A **pool** keeps several database connections ready to go. Instead of opening a new connection per request (slow), handlers borrow one from the pool, use it, and return it.

#### Migrations

```rust
sqlx::migrate!("../../migrations").run(pool).await?;
```

This reads the `.sql` files in `migrations/` and runs them in order. The `migrate!` macro embeds the SQL into the binary at compile time, so you don't need the migration files deployed alongside the binary.

#### The Repo Pattern

Each table gets a module (`app_repo.rs`, `deployment_repo.rs`, etc.) with functions like `create`, `get_by_id`, `list`, `delete`. Line by line:

```rust
pub async fn get_by_name(pool: &SqlitePool, name: &str) -> Result<App, HomelabError> {
    let row = sqlx::query_as::<_, AppRow>("SELECT * FROM apps WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(|e| HomelabError::Database(e.to_string()))?
        .ok_or_else(|| HomelabError::NotFound(format!("app not found: {name}")))?;

    Ok(row.into())
}
```

| Part | What it does |
|------|-------------|
| `async fn` | This function is asynchronous — it can pause while waiting for I/O |
| `&SqlitePool` | A **reference** (borrow) to the pool, not taking ownership |
| `-> Result<App, HomelabError>` | Returns either an App or an error |
| `query_as::<_, AppRow>(...)` | Run SQL and map the result to an `AppRow` struct |
| `.bind(name)` | Fill in the `?` placeholder (prevents SQL injection) |
| `.fetch_optional` | Returns `Option<Row>` — `Some(row)` or `None` |
| `.await` | Pause here until the database responds |
| `.map_err(...)` | Convert sqlx's error type into our `HomelabError` |
| `?` | The **question mark operator** — if this is `Err`, return it immediately. If `Ok`, unwrap the value |
| `.ok_or_else(...)` | Convert `None` into `Err(NotFound)` |
| `row.into()` | Convert the raw DB row into our `App` type |

#### The AppRow → App Conversion

SQLite stores everything as basic types (TEXT, INTEGER). We need to convert between DB rows and our domain types:

```rust
#[derive(sqlx::FromRow)]
struct AppRow {
    status: String,  // DB stores "running" as text
    // ...
}

impl From<AppRow> for App {
    fn from(row: AppRow) -> Self {
        Self {
            status: row.status.parse().unwrap_or(AppStatus::Created),
            // ...
        }
    }
}
```

`FromRow` auto-maps DB columns to struct fields. The `From` impl converts the raw string `"running"` into the type-safe `AppStatus::Running` enum. This keeps sqlx as a dependency of `homelab-db` only — `homelab-core` stays clean.

---

### homelab-docker — Docker Integration

**Location:** `crates/homelab-docker/src/`

Uses **bollard**, a Rust library that talks to the Docker daemon via its Unix socket (`/var/run/docker.sock` — the same socket the `docker` CLI uses).

#### client.rs

```rust
pub fn connect() -> Result<Docker, HomelabError> {
    Docker::connect_with_socket_defaults()
        .map_err(|e| HomelabError::Docker(format!("failed to connect: {e}")))
}
```

Creates a connection to the local Docker daemon. This is equivalent to running `docker` commands, but programmatically.

#### containers.rs

The `create_and_start` function is the core. It:

1. **Removes** any existing container with that name (so redeploys replace cleanly)
2. **Builds a config** with the Docker image, environment variables, Traefik labels, and network
3. **Creates** the container, then **starts** it

The container naming convention is `homelab-<app-name>` (e.g., `homelab-my-app`). This makes it easy to identify PaaS-managed containers vs other Docker containers.

#### labels.rs — Traefik Auto-Discovery

```rust
HashMap::from([
    ("traefik.enable", "true"),
    ("traefik.http.routers.homelab-myapp.rule", "Host(`myapp.lab.example.com`)"),
    ("traefik.http.services.homelab-myapp.loadbalancer.server.port", "3000"),
])
```

When Traefik sees these labels on a running container, it automatically creates a route: "if the `Host` header matches `myapp.lab.example.com`, forward traffic to this container on port 3000." Zero config files needed — labels are the config.

#### logs.rs

Docker returns logs as an **async stream** — data arrives piece by piece rather than all at once. We use `futures_util::StreamExt` to collect it into a `Vec<String>`:

```rust
let mut stream = docker.logs(&name, Some(opts));
while let Some(result) = stream.next().await {
    match result {
        Ok(output) => lines.push(output.to_string()),
        Err(e) => return Err(...),
    }
}
```

`while let Some(result) = stream.next().await` means: "keep pulling items from the stream until it's empty."

---

### homelab-api — HTTP Server

**Location:** `crates/homelab-api/src/`

Uses **axum**, the standard Rust async web framework (built by the Tokio team).

#### main.rs — Entry Point

The `#[tokio::main]` attribute sets up the async runtime:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Init structured logging
    // 2. Read config from env vars
    // 3. Connect to SQLite + run migrations
    // 4. Connect to Docker
    // 5. Build AppState
    // 6. Build router + start listening on port 3001
}
```

`anyhow::Result<()>` means "this can return any error type." `anyhow` is the counterpart to `thiserror` — used in the top-level binary where you just want to propagate errors and print them, not type-match on every variant.

#### state.rs — Shared State

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub docker: Docker,
    pub config: AppConfig,
}
```

axum clones this and passes it to every request handler. Cloning is cheap — `SqlitePool` and `Docker` are internally reference-counted (like a shared pointer), so cloning just increments a counter.

#### router.rs — Route Mapping

```rust
Router::new()
    .route("/apps", get(handlers::apps::list).post(handlers::apps::create))
    .route("/apps/{name}", get(handlers::apps::get)
                              .put(handlers::apps::update)
                              .delete(handlers::apps::delete))
    .route("/apps/{name}/start", post(handlers::containers::start))
```

Maps HTTP method + path → handler function. `{name}` is a path parameter (like Express's `:name`).

#### Handlers

Handlers are async functions with a special signature that axum understands:

```rust
pub async fn create(
    State(state): State<AppState>,       // axum injects shared state
    Json(req): Json<CreateAppRequest>,   // axum parses JSON body
) -> Result<Json<ApiResponse<App>>, ApiError> {
```

axum uses Rust's type system as the framework. The argument types tell axum what to extract:
- `State(state)` — pull the shared `AppState` from the router
- `Json(req)` — parse the request body as JSON into `CreateAppRequest`
- `Path(name)` — extract the `{name}` path parameter

The return type says "either return JSON or an API error." axum calls `IntoResponse` on whatever you return.

#### error.rs — API Response Envelope

Every response follows this shape:

```json
{ "success": true,  "data": { ... }, "error": null   }
{ "success": false, "data": null,    "error": "not found: my-app" }
```

`ApiError` implements axum's `IntoResponse` trait to map our error types to HTTP status codes:

| HomelabError | HTTP Status |
|-------------|-------------|
| `NotFound` | 404 |
| `AlreadyExists` | 409 Conflict |
| `InvalidInput` | 400 Bad Request |
| `Docker` | 500 Internal Server Error |
| `Database` | 500 Internal Server Error |

---

## Rust Concepts Reference

### Ownership & Borrowing

Rust's signature feature. Every value has exactly **one owner**. When the owner goes out of scope, the value is freed. You can lend values via references:

- `&T` — shared (read-only) reference. Multiple can exist at once.
- `&mut T` — exclusive (mutable) reference. Only one can exist.

This eliminates null pointers, dangling pointers, use-after-free, and data races — all **at compile time**, with zero runtime cost.

```rust
let name = String::from("my-app");
let r = &name;         // borrow (read)
println!("{}", r);     // fine
println!("{}", name);  // also fine — original still valid

let name2 = name;      // MOVE — name is now invalid
// println!("{}", name); // compile error: value moved
```

### String vs &str

| Type | What it is | When to use |
|------|-----------|-------------|
| `String` | Owned, heap-allocated, growable | When you need to store or modify a string |
| `&str` | Borrowed reference to string data | Function parameters that just need to read |

Functions that don't need to own the string take `&str`, so callers can pass either a `String` (auto-borrows) or a string literal.

### Option\<T\> — No Null

Rust has no `null`. Instead, a value that might be missing is `Option<T>`:

```rust
let found: Option<App> = fetch_optional(pool).await;

match found {
    Some(app) => println!("got {}", app.name),
    None => println!("not found"),
}

// Or use combinators:
let name = found.map(|app| app.name).unwrap_or("unknown".into());
```

The compiler forces you to handle both cases. You literally cannot access the inner value without first checking if it exists.

### Result\<T, E\> — No Exceptions

Functions that can fail return `Result`:

```rust
fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("division by zero".into())
    } else {
        Ok(a / b)
    }
}
```

The `?` operator is shorthand for "if error, return it; if ok, unwrap":

```rust
let x = divide(10.0, 2.0)?;  // x = 5.0, or function returns early with error
```

### async/await

Same concept as JavaScript. Functions that do I/O are `async` and you `.await` their result:

```rust
async fn fetch_app(name: &str) -> Result<App, Error> {
    let app = db.query("SELECT ...").await?;  // pause here until DB responds
    Ok(app)
}
```

Under the hood, **Tokio** (the async runtime) manages a thread pool and schedules tasks efficiently. While one request awaits a DB query, another request can use that thread.

### Traits

Traits are like interfaces — they define behavior that types can implement:

```rust
// The trait (defined by serde)
trait Serialize {
    fn serialize(&self, ...) -> ...;
}

// We implement it via derive
#[derive(Serialize)]
struct App { ... }  // Now App can be serialized to JSON
```

Key traits in our codebase:
- `Serialize` / `Deserialize` — JSON conversion
- `FromRow` — DB row to struct conversion
- `IntoResponse` — struct to HTTP response conversion
- `From<A> for B` — converts type A into type B (used with `.into()`)
- `Display` — how a type renders as a string (used with `format!`, `println!`)

### The ? Operator

Rust's most-used shorthand. These are equivalent:

```rust
// With ?
let app = get_by_name(pool, name).await?;

// Without ? (what the compiler expands it to)
let app = match get_by_name(pool, name).await {
    Ok(val) => val,
    Err(e) => return Err(e.into()),
};
```

---

## Further Reading

### Rust Language

- [The Rust Book](https://doc.rust-lang.org/book/) — the official guide, free and thorough. Start here.
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) — learn through annotated code examples
- [Rustlings](https://github.com/rust-lang/rustlings) — small exercises to practice syntax and concepts
- [The Cargo Book](https://doc.rust-lang.org/cargo/) — everything about Cargo, workspaces, and dependencies

### Libraries We Use

- [axum docs](https://docs.rs/axum/latest/axum/) — the web framework, with examples for routing, extractors, and middleware
- [sqlx docs](https://docs.rs/sqlx/latest/sqlx/) — async SQL with compile-time checked queries
- [bollard docs](https://docs.rs/bollard/latest/bollard/) — Rust Docker API client
- [serde docs](https://serde.rs/) — the serialization framework (JSON, TOML, YAML, etc.)
- [tokio docs](https://tokio.rs/) — the async runtime that powers everything
- [thiserror docs](https://docs.rs/thiserror/latest/thiserror/) — ergonomic custom error types
- [anyhow docs](https://docs.rs/anyhow/latest/anyhow/) — flexible error handling for applications

### Concepts Deep Dives

- [Understanding Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html) — the ownership/borrowing system
- [Error Handling in Rust](https://doc.rust-lang.org/book/ch09-00-error-handling.html) — Result, Option, and the ? operator
- [Async Programming in Rust](https://rust-lang.github.io/async-book/) — how async/await works under the hood
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — conventions for writing idiomatic Rust

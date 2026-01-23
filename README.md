# Parv – WebAssembly UI Toolkit

**Parv** is a lightweight **WebAssembly UI toolkit** for building dynamic web interfaces using **Rust + HTML templates**, with minimal JavaScript and a simple, explicit data & element binding model.

> Parv focuses on *direct DOM control*, *explicit rendering*, and *clear mental models* rather than virtual DOMs or heavy abstractions.

---

## What Is Parv?

Parv is best described as a **WebAssembly UI toolkit**:

* UI is defined using an **HTML string** (from `tx!` macro or template elements)
* Templates are **mounted to a DOM node** (e.g. `body`)
* Elements can be **tagged and accessed** from Rust via `pv-tag`
* Data bound to templates can be retrieved via `Context::data()` (deserialized into the original struct)
* Rendering is **explicit** and predictable
* No virtual DOM, no diffing, no reactive runtime

---

## 1. UI Sources: Strings, Template Macro, or HTML `<template>`

Parv allows defining UI in **three equivalent ways**:

### a) HTML String (`tx!` macro)

The `tx!` macro returns a **string of HTML**:

```rust
let template = tx!(
    <div>
        <h1 pv-tag="title"></h1>
    </div>
);
```

### b) Raw HTML string

You can also render UI directly from a raw HTML string:

```rust
"<h1>Hello</h1>".mount_on_body();
```

### c) Template element in HTML

If you have a `<template>` in your HTML file:

```html
<template id="content">
  <div>
    <h1 pv-tag="title"></h1>
  </div>
</template>
```

You can load it in Rust:

```rust
let template = Template::from_id("content");
```

---

## 2. Mounting

Parv provides **three explicit mounting methods** that define where UI is rendered:

```rust
mount_on(element)
mount_on_body()
mount_on_id("app")
```

#### `mount_on(Element)`

Mounts the UI on a specific DOM element.

```rust
template.mount_on(container_element);
```

#### `mount_on_body()`

Mounts the UI directly to `<body>`.

```rust
template.mount_on_body();
```

#### `mount_on_id(String)`

Mounts the UI to an element by its DOM id.

```rust
template.mount_on_id("app".into());
```

---

## 3. Element Tagging (`pv-tag`)

Elements inside the template can be tagged using the `pv-tag` attribute:

```html
<h1 pv-tag="title"></h1>
```

At runtime:

* All `pv-tag` elements are collected
* Stored in a `HashMap<String, Element>`
* Made available through a `Context`

This allows **direct and type-safe DOM access from Rust**.

---

## 4. Callback-Based UI Logic

UI logic is defined using `with_callback`:

```rust
template
    .with_callback(move |ctx: Context| {
        ui!(ctx.ui(), title);
        title.set_inner_html("Hello World");
    })
    .render();
```

Inside the callback:

* You receive a `Context`
* Tagged elements are accessed via `ctx.ui()`
* Bound data can be accessed via `ctx.data()`

The `ui!` macro extracts elements from the internal hashmap and binds them to local variables.

---

## 5. Explicit Rendering

Rendering does **not** happen automatically.

```rust
template.render();
```

This design ensures:

* Predictable performance
* No hidden re-renders
* Full control over when DOM updates occur

---

## 6. Clearing the UI

To remove all mounted content:

```rust
template.clear();
```

This clears the mount point completely.

---

## 7. Data Binding (`with_data`)

Parv supports **one-way data binding** using serializable data.

### Example: Binding a `User` Struct

```rust
use serde::Serialize;

#[derive(Serialize)]
struct User {
    username: String,
    email: String,
}

let template = tx!(
    <div>
        <h2 pv-text="username"></h2>
        <p pv-text="email"></p>
    </div>
);

let user = User {
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
};

template
    .mount_on_body()
    .with_data(user)
    .render();
```

Result:

* `username` is inserted into the `<h2>` element
* `email` is inserted into the `<p>` element
* Updates flow **one-way: Rust → DOM**
* Data can be retrieved in callbacks using `Context::data()`.

You can retrieve the bound data at any time in the callback:

```rust
template.with_callback(|ctx: Context| {

    let user_result: Result<User, serde_json::Error> = ctx.data();
});
```

Rules:

* Field names must match `pv-text` values
* No two-way binding or reactive updates

---

## Parv Design Philosophy

Parv intentionally avoids:

* Virtual DOMs
* Implicit reactivity
* Heavy framework abstractions

Instead, it emphasizes:

* Direct DOM access
* Explicit lifecycles
* Wasm-native performance

---

## When to Use Parv

Use Parv if you want:

* A Rust-first Wasm UI layer
* Fine-grained DOM control
* Predictable rendering behavior

---

## Status

⚠️ Early-stage / experimental

APIs may change as the design evolves.

---

## License

Apache-2.0

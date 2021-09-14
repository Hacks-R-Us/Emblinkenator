# Protected Id

Protected Id is a macro for creating type-checked Id strings. This is intended to prevent errors caused by storing Ids as strings, and to provide a readonly interface to Ids.

```rust
#[macro_use]
extern crate protected_id_derive;

#[derive(ProtectedId)]
struct SomeIdType {
    #[protected_value]
    id: String
}

#[derive(ProtectedId)]
struct SomeOtherIdType {
    #[protected_value]
    id: String
}

fn do_something (id: &SomeIdType) {
    // do something here
}

// Compiles
let id = SomeIdType::new();
do_something(&id);

// Does not compile
let id = SomeOtherIdType::new();
do_something(&id);

```

The stored Id can be retrieved as a `String` by calling `id.unprotect()`.

```rust
#[macro_use]
extern crate protected_id_derive;

#[derive(ProtectedId)]
struct SomeIdType {
    #[protected_value]
    id: String
}

let id = SomeIdType::new();
let stored_id: String = id.unprotect();
```

A protected Id can also be constructed from an existing `String` (e.g. reading data from a database into a struct) by using the `new_from(id: String)` associated function.

## Requirements

Any crate that uses protected_id must depend on the [uuid](https://crates.io/crates/uuid) crate

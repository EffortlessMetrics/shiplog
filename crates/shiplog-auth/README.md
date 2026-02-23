# shiplog-auth

Authentication and authorization utilities for shiplog.

## Overview

This crate provides authentication and authorization primitives for the shiplog project, including:
- Permission levels (Read, Write, Admin)
- Identity management
- Authorization context
- API token support

## Usage

```rust
use shiplog_auth::{Identity, Permission, AuthContext};

// Create an identity with permissions
let identity = Identity::new("user1", "John Doe")
    .with_permission(Permission::Read)
    .with_permission(Permission::Write);

// Create an authorization context
let ctx = AuthContext::new(identity);

// Check permissions
if ctx.can(Permission::Write) {
    println!("User can write!");
}
```

## License

MIT OR Apache-2.0

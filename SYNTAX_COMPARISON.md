# Syntax Comparison: Production vs Testing

## The Problem (Before)

Previously, anchor-litesvm had divergent syntax from production code:

```rust
// TESTING with old anchor-litesvm (divergent syntax)
let ix = ctx.instruction()
    .accounts(my_program::client::accounts::Transfer { ... })
    .args(my_program::client::args::Transfer { ... })
    .build();

// PRODUCTION with anchor-client (different!)
let ix = program
    .request()
    .accounts(my_program::accounts::Transfer { ... })
    .args(my_program::instruction::Transfer { ... })
    .instructions()?[0];
```

Developers had to learn two different patterns - one for tests, one for production.

## The Solution (Now)

Now anchor-litesvm uses the **exact same syntax** as production anchor-client:

```rust
// TESTING with new anchor-litesvm
let ix = ctx.program()
    .request()
    .accounts(my_program::client::accounts::Transfer { ... })
    .args(my_program::client::args::Transfer { ... })
    .instructions()?[0];

// PRODUCTION with anchor-client (identical!)
let ix = program
    .request()
    .accounts(my_program::client::accounts::Transfer { ... })
    .args(my_program::client::args::Transfer { ... })
    .instructions()?[0];
```

## Benefits

1. **Zero Learning Curve**: If you know anchor-client, you already know how to use anchor-litesvm
2. **Copy-Paste Compatible**: Code can be moved between tests and production without modification
3. **Ecosystem Alignment**: Follows established Anchor patterns
4. **No RPC Overhead**: Still native implementation, just with familiar syntax
5. **Documentation Reuse**: All anchor-client tutorials and examples work with our test framework

## Implementation

The native `Program` and `RequestBuilder` in anchor-litesvm provide the same API surface as anchor-client, but:
- No RPC connections needed
- No mock clients to set up
- No network dependencies
- Faster compilation
- Direct integration with LiteSVM

## Migration

If you were using the old anchor-litesvm syntax:
```rust
// Old
ctx.instruction()
    .accounts(...)
    .args(...)
    .build()

// New (production-compatible)
ctx.program()
    .request()
    .accounts(...)
    .args(...)
    .instructions()?[0]
```

The syntax is now identical to what you'd use with anchor-client in production!
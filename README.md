# Sodium
A Functional Reactive Programming (FRP) library for Rust

Avaliable on crates.io: https://crates.io/crates/sodium-rust

See tests under src/tests for example usage. Sodium objects within lambda expressions are traced via lambda1, lambda2, etc. just like the TypeScript version does.

## Pitfalls

### No Global State

You must create a SodiumCtx for your application and keep passing it around in order to create sodium objects.

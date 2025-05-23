[![License: Apache-2.0](https://img.shields.io/badge/Apache%202.0-23454d?logo=apache)](./LICENSE)

# fixed

A generic fixed point wrapper around the `U256` type from ethers-rs.

The math in this library is a heavily based on Solidity's FixedPointMath library with a few changes:

- The outward type of the underlying value is generic, allowing the library to be used with any type that implements `FixedPointValue`, including signed integers, and ensuring that the instance is bounded by the generic type's limits.
- Support for overflowing intermediate operations in `mul_div_down` and `mul_div_up` via `U512`.
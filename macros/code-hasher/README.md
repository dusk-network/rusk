![Build Status](https://github.com/dusk-network/rusk/workflows/Continuous%20integration/badge.svg)
[![Repository](https://img.shields.io/badge/github-code--hasher-blueviolet?logo=github)](https://github.com/dusk-network/code-hasher)
[![Documentation](https://img.shields.io/badge/docs-code--hasher-blue?logo=rust)](https://docs.rs/code-hasher/)
# code-hasher

Tiny proc macro library designed to hash a code block generating a unique
identifier for it which will get written into a `const` inside of the code
block.

## Example

```rust
#[code_hasher::hash(SOME_CONST_NAME, version = "0.1.0")]
pub mod testing_module {
    pub fn this_does_something() -> [u8; 32] {
        SOME_CONST_NAME
    }
}
```

Here, `SOME_CONST_NAME` has assigned as value the resulting hash of:
- The code contained inside `testing_module`.
- The version passed by the user (is optional). Not adding it will basically
  not hash this attribute and **WILL NOT** use any default alternatives.

  
## Licensing
This code is licensed under Mozilla Public License Version 2.0 (MPL-2.0).
Please see [LICENSE](https://github.com/dusk-network/rusk/tree/master/macros/code-hasher/LICENSE) for further info.
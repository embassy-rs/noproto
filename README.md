# noproto

No-`std`, no-`alloc` protocol buffers (protobuf) implementation in Rust, for embedded systems.
Optimized for binary size and memory usage, not for performance.

Status: **very experimental** :radioactive: 

## Features

Implemented:

- Derive macros.
- `heapless::Vec`, `heapless::String` impls.
- `optional`
- `repeated`
- `oneof`
- `enum`

Not implemented (yet?):

- Impls for `alloc` containers (goal is to be `no-alloc`, but we could still have them optionally).
- Some types (see below)
- Impls for `&[T]` for repeated fields (only doable for writing, not reading)
- Tool to compile `.proto` files into Rust code.
- Maps
- Deprecated field groups.

## Type mapping

| Protobuf | Rust | 
|-|-|
| `bool` | bool |
| `int32` | TODO |
| `uint32` | `u32` |
| `sint32` | `i32` |
| `fixed32` | TODO |
| `sfixed32` | TODO |
| `int64` | TODO |
| `uint64` | `u64` |
| `sint64` | `i64` |
| `fixed64` | TODO |
| `sfixed64` | TODO |
| `float` | TODO |
| `double` | TODO |
| `string` | `heapless::String<N>` |
| `bytes` | `heapless::Vec<u8, N>` |

## Minimum supported Rust version (MSRV)

`noproto` is guaranteed to compile on the latest stable Rust version at the time of release. It might compile with older versions but that may change in any new patch release.

## License

This work is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

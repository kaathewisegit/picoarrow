`picoarrow` is a tiny library for creating Apache Arrow arrays and
writing serializing them into the Arrow IPC format.  It has three main
differences from the [mainline `arrow` crate][a]:

- I tried to pull in minimal amount of dependencies possible.  The core
  modules only rely on `bytemuck` for casting primitive buffers to
  `[u8]` and `flatbuffers` for IPC support.  Additional libraries
  necessary for half-floats and compression are feature-gated.

- `picoarrow` is a bit more type-safe than `arrow`.  All array types are
  parametrized, meaning `List<Int64>` and `List<UInt64>` are represented
  by two different Rust types.  So, there should generally be no need
  for `Box<dyn Array>` and downcasting[^dyn].

- Because of the type restrictions and the fact it's a tiny library
  which is only focused on what I need, `picoarrow` doesn't support
  structs and unions, a number of primitive types such as dates and
  decimals, and various functionality like reading IPC streams.


[^dyn]: `dyn` pointers are used to write arrays into batches, though.

[a]: https://lib.rs/arrow

# Architecture

## Source

- `array/`

  - `mod.rs` defines the common array trait and re-exports all of the concrete array implementations.
  - `variable` defines binary and UTF-8 arrays.
  - `fixed_list` defines `FixedSizeList`.
  - `fixed_binary` defines `FixedSizeBinary`.
  - `primitive` defines `ArrayPrimitive`, which wraps all uniform-sized copyable Arrow types which don't have any array-associated data (like decimals or time units).
  - `boolean` defines boolean array, as it uses a bitmap for storage.

- `bitmap` defines a `ValidityBuffer` trait which is used to implement validity bitmaps.

- `fb/` contains generated Flatbuffers code for the IPC encoding.

- `error` defines the `Error` type used throughout the crate.

- `schema` describes

- `ipc/` implements the Arrow IPC encoding.

  - `mod.rs` re-exports writers and defines `Compression`.
  - `stream` implements the stream writer
  - `file` implements the file writer, which is a thin wrapper over the stream writer with record batch offset tracking.


## Tests

All tests live in a single binary defined in `tests/it/main.rs` to speed up compilation times.

Where possible tests use the `arbitrary` crate together with `arbtest` to randomize the input data.

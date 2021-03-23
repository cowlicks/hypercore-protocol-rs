# Changelog

All notable changes to this Rust implementation of hypercore-protocol will be documented here.

### unreleased

* Emit errors when trying to send on a channel after it was closed by either side
* Remove the arument to `channel.close()`, because it is not needed

### 0.3.0

#### API breaking changes

* Changed the generic argument of the Protocol struct to be a single IO handle (AsyncRead + AsyncWrite) in place of a R: AsyncRead and W: AsyncWrite. This is a change in the public API. If you have seperate reader and writer structs, a Duplex handle is provided that combines the reader and writer. If you have a struct that is both AsyncRead and AsyncWrite, the Protocol is now generic just over that struct.
* Reworked internals to use manual poll functions and not an async function. The `Protocol` struct now directly implements `Stream`, the `ProtocolStream` wrapper is removed.
* Made the `Event` enum non-exhaustive.
* Removed the `destroy` method, errors are emitted on the protocol stream.
* Changed key and discovery key values to be `[u8; 32]` in place of `Vec<u8>`
  > . To convert from a `Vec<u8>`, use `key.try_into().unwrap()` if you're sure that the key is a 32 byte long `u8` vector.

### 0.0.2

initial release

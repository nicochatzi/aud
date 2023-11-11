# Crate Layout

- [`lib`](./lib): Core library for stream manipulations, Lua engine embedding, etc..
- [`ui`](./ui):  UI library that is independent of `lib`
- [`cli`](./cli): App that uses `lib` and `ui` to build the `aud` CLI client

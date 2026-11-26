# tf_rs

A TF2 cheat for Linux written in Rust, meant as an educational reference for shared library injection, vtable hooking and Source engine internals.

> Only use it where you're allowed to modify the game client: private servers, single player and the like.

## Building

```bash
sudo apt install libsdl2-dev clang gdb libglew-dev
```

GLEW **2.1** is required exactly: the library links against `libGLEW.so.2.1`, and 2.2 doesn't work with the Steam Runtime. You may need to build it from source.

With the game running:

```bash
cargo inject            # build and inject
cargo inject --release
cargo inject --debug    # also open a terminal tailing the log
```

## Documentation

Hosted at [faluthe.github.io/tf_rs](https://faluthe.github.io/tf_rs/), built from `main`. To build it locally:

```bash
cargo doc --open
```

## Dependencies

| Dependency    | Used for                                                  |
|---------------|-----------------------------------------------------------|
| `anyhow`      | Error handling                                            |
| `log`         | Logging, written to `tf_rs.log` next to the injected `.so` |
| `libc`        | `dlopen`, `dlsym`, `dladdr`, pthreads                     |
| `once_cell`   | Lazily initialized statics                                |
| `serde`       | Reading cargo's JSON output in the injector               |
| `nuklear`     | Safe wrapper around Nuklear for the menu                  |
| `nuklear_sys` | Nuklear C bindings, built with `bindgen`                  |
| SDL2          | The game's window and input, hooked for the menu          |
| GLEW 2.1      | OpenGL loading for the menu's GL context                  |
| GDB           | Calling `dlopen` in the game to inject                    |

# LAYER-W - Near Native Web Execution Layer for Games & Applications

## LAYER-W - Engine Architecture

- Networking: HTTP via wstd and WASI HTTP APIs
- Input Handling: wINPUT for unified input management. Custom input events for gamepad, keyboard, and mouse. Maps seamlessly from native to web environments.
- Audio: https://crates.io/crates/geng-web-audio-api/src/platform/web.rs
- Graphics: WebGPU for rendering, (GLFW for windowing). Uses wgpu crate for WebGPU bindings.
- Host Runtimes for Components
  The wit-bindgen project is intended to facilitate in generating a component, but once a component is in your hands the next thing to do is to actually execute that somewhere. This is not under the purview of wit-bindgen itself but these are some resources and runtimes which can help you work with components:

  - Rust: the wasmtime crate is an implementation of a native component runtime that can run any WIT world. It additionally comes with a bindgen! macro which acts similar to the generate! macro in this repository. This macro takes a WIT package as input and generates trait-based bindings for the runtime to implement and use.

  - JS: the jco project can be used to execute components in JS either on the web or outside the browser in a runtime such as node. This project generates a polyfill for a single concrete component to execute in a JS environment by extracting the core WebAssembly modules that make up a component and generating JS glue to interact between the host and these modules.

- Main Loop: cdylib + wit as an IDL
  - The wit-bindgen project extensively uses WIT definitions to describe imports and exports. The items supported by WIT directly map to the component model which allows core WebAssembly binaries produced by native compilers to be transformed into a component. All imports into a WebAssembly binary and all exports must be described with WIT.

```rust
// wit/host.wit
package example:host;

world host {
  import print: func(msg: string);

  export run: func();
}

// src/lib.rs

// Use a procedural macro to generate bindings for the world we specified in
// `host.wit`
wit_bindgen::generate!({
    // the name of the world in the `*.wit` input file
    world: "host",
});

// Define a custom type and implement the generated `Guest` trait for it which
// represents implementing all the necessary exported interfaces for this
// component.
struct MyHost;

impl Guest for MyHost {
    fn run() {
        print("Hello, world!");
    }
}

// export! defines that the `MyHost` struct defined below is going to define
// the exports of the `world`, namely the `run` function.
export!(MyHost);
```

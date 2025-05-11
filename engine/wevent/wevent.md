# SUBMODULE_002: WEVENT: A WebAssembly friendly event system using Rust

To address different event models between web and native:

- Create a normalized event queue that collects from both sources
- Implement event translation layers for platform-specific inputs
- Provide a consistent timing model for event handling

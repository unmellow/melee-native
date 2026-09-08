# Native Linux port

This fork adds a **host** under [`native/`](native/README.md) so the decompiled C can target a desktop instead of `main.dol`.

- Renderer: **Vulkan** through **wgpu** (Rust)
- Input: native **PAD** — Wii U GameCube adapter, Xbox / DualShock / Switch Pro via Linux joystick, keyboard
- Build: `cargo run --manifest-path native/Cargo.toml --release`

The matching GameCube build (`python configure.py && ninja`) is unchanged.

See [native/README.md](native/README.md) for controller maps, `MELEE_DATA`, and `configure_native.py`.

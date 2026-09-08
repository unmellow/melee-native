# Melee Native — Linux host

Vulkan renderer (wgpu) + native controllers for the [doldecomp/melee](https://github.com/doldecomp/melee) C source.

This is the **host layer** the matching decomp never had: GX, PAD, OS, VI, and DVD implemented against Linux so recovered C can be compiled with gcc into a real executable instead of `main.dol`.

You still need a **legally obtained US 1.02 (GALE01)** disc dump for assets. Nothing from the disc is in this tree.

## What works today

| Layer | Status |
|---|---|
| `melee-native` Linux binary | yes — Vulkan window (feature `window`), 640×480 EFB, 60 Hz |
| Headless GX + PAD | `cargo run --no-default-features -- --headless` |
| GX immediate mode → wgpu | triangles / quads / fans / TEV modulate-replace-decal, RGBA8/CMPR |
| PAD 4 ports | Wii U GCC adapter (hidraw), `/dev/input/js*`, keyboard fallback |
| Analog + rumble | GCC analog sticks / L-R analog; rumble on the adapter |
| OS / VI / DVD | time, report, retrace; DVD paths under `MELEE_DATA` |
| Full decomp linked as the game | **in progress** — `configure_native.py` schedules gcc; Gekko asm / MWCC TUs still need shims |

## Build

Needs: Rust 1.80+, gcc. For the Vulkan window: a Vulkan ICD (`vulkaninfo`) plus `pkg-config`, `libwayland-dev`, `libxkbcommon-dev` (or an X11 stack).

```bash
cd native
cargo test --no-default-features          # PAD + GX unit tests, no GPU
cargo run --no-default-features -- --headless
cargo run --release                       # Vulkan window (default feature `window`)
```

Binary: `native/target/release/melee-native`.

Keyboard (port 1 if no pad is plugged in):

| Action | Key |
|---|---|
| Stick | WASD |
| C-stick | TFGH |
| A B X Y | Space, Shift, X, C |
| L R Z Start | Q, E, Z, Enter |
| D-pad | arrows |

GameCube controllers through a **Wii U adapter**:

```bash
sudo cp native/udev/51-gcadapter.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
```

Xbox, DualShock 4/5, Switch Pro, and 8BitDo devices show up as `/dev/input/js*` and are mapped onto `PADStatus`.

`MELEE_DATA` should point at an extracted GALE01 filesystem (Dolphin → Properties → Filesystem → Extract).

## Linking the decomp later

```bash
python native/configure_native.py
ninja -f build-native.ninja            # host lib first
# then chip through skipped TUs and:
ninja -f build-native.ninja melee-native-game
```

`configure_native.py` **excludes** `extern/dolphin/src` (replaced by this crate), MetroTRK, MSL, and the Gekko runtime. Remaining C is compiled with `-DMELEE_NATIVE` and linked against `libmelee_host.a` (`PADInit`, `PADRead`, `GXBegin`, `OSGetTime`, `DVDOpen`, …).

## Layout

```
native/
  host/           Rust crate (staticlib + binary)
    src/gx.rs     GX command processor
    src/renderer.rs   wgpu Vulkan EFB
    src/pad.rs    PADStatus backends
    src/os.rs     OS / VI
    src/dvd.rs    disc as a directory
    src/ffi.rs    C ABI the decomp already calls
  include/        host-only header
  udev/           GCC adapter rule
  configure_native.py
```

Endian conversion of DAT archives, ARAM, DSP/AX, and a complete TEV compiler are still open work. Fill those in here — do not touch MWCC matching code on `master`.

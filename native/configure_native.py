#!/usr/bin/env python3
"""Generate a ninja file that compiles decomp C with a host gcc and links
libmelee_host.a (wgpu Vulkan + PAD).

This is the native-port configure step. It does *not* use MWCC / wibo.
Many translation units still contain Gekko asm or MWCC-only constructs; those
are recorded as skipped so you can chip away at them.

Usage (from repo root):

    python native/configure_native.py
    ninja -f build-native.ninja melee-native-game   # once enough TUs compile
    cargo run --manifest-path native/Cargo.toml --release
"""

from __future__ import annotations

import os
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
NATIVE = ROOT / "native"

SKIP_DIRS = {
    "src/MetroTRK",
    "src/Runtime",
    "src/MSL",
    "extern/dolphin/src",  # replaced by Rust host
    "extern/dolphin/nonmatchings",
}

CC = os.environ.get("CC", "gcc")
CFLAGS = [
    "-O2",
    "-g",
    "-fno-strict-aliasing",
    "-fwrapv",
    "-std=gnu11",
    "-DMELEE_NATIVE=1",
    "-DNATIVE=1",
    "-DGEKKO=0",
    "-I",
    str(ROOT / "src"),
    "-I",
    str(ROOT / "extern/dolphin/include"),
    "-I",
    str(NATIVE / "include"),
    "-Wno-unused-variable",
    "-Wno-unused-function",
    "-Wno-pointer-sign",
]


def should_skip(path: pathlib.Path) -> bool:
    rel = path.relative_to(ROOT).as_posix()
    return any(rel.startswith(d) for d in SKIP_DIRS)


def collect_c() -> list[pathlib.Path]:
    files: list[pathlib.Path] = []
    for folder in (ROOT / "src", ROOT / "extern"):
        if not folder.exists():
            continue
        for p in folder.rglob("*.c"):
            if should_skip(p):
                continue
            files.append(p)
    return sorted(files)


def main() -> int:
    sources = collect_c()
    out = ROOT / "build-native.ninja"
    lines = [
        "ninja_required_version = 1.8",
        f"cc = {CC}",
        "cflags = " + " ".join(CFLAGS),
        "",
        "rule cc",
        "  command = $cc -c $cflags -o $out $in",
        "  description = CC $in",
        "",
        "rule cargo_host",
        "  command = cargo build --release --manifest-path native/Cargo.toml",
        "  description = CARGO melee-host",
        "",
        "build native/target/release/libmelee_host.a: cargo_host",
        "",
    ]
    objs: list[str] = []
    for src in sources:
        rel = src.relative_to(ROOT).as_posix()
        obj = f"build/native-obj/{rel}.o"
        lines.append(f"build {obj}: cc {rel}")
        objs.append(obj)

    lines += [
        "",
        "rule link",
        "  command = $cc -o $out $in native/target/release/libmelee_host.a "
        "-lpthread -ldl -lm -lstdc++",
        "  description = LINK $out",
        "",
        "build melee-native-game: link "
        + " ".join(objs)
        + " | native/target/release/libmelee_host.a",
        "",
        f"# {len(sources)} C translation units scheduled.",
        "# Expect failures on Gekko asm / MWCC-only TUs; comment them out of",
        "# this file or add MELEE_NATIVE shims as you go.",
        "default native/target/release/libmelee_host.a",
        "",
    ]
    out.write_text("\n".join(lines))
    print(f"wrote {out} ({len(sources)} C files)")
    print("next: cargo build --release --manifest-path native/Cargo.toml")
    print("      ninja -f build-native.ninja")
    return 0


if __name__ == "__main__":
    sys.exit(main())

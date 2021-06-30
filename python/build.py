#!/usr/bin/env python3

import os
import sys
import platform
from typing import Optional
import subprocess


def shell(cmd: str):
    print(f">> {cmd}\n")
    return subprocess.run(cmd, shell=True)


def get_version_number() -> Optional[str]:
    with open("Cargo.toml", "r") as myfile:
        lines = myfile.readlines()
    for line in lines:
        if "version" in line:
            (_k,v) = line.split("=")
            v = v.strip().strip('"')
            return v
    return None

# See https://en.wikipedia.org/wiki/X86_Bit_manipulation_instruction_set

# rustc --print target-features
# rustc --print cfg
# rustc --print=cfg -C target-cpu=native
MODERN = "+bmi,+popcnt,+lzcnt"

GENERIC = ""

# athlon64
# $ rustc --print=cfg -C target-cpu=native
# debug_assertions
# target_arch="x86_64"
# target_endian="little"
# target_env="msvc"
# target_family="windows"
# target_feature="aes"
# target_feature="avx"
# target_feature="bmi1"
# target_feature="fma"
# target_feature="fxsr"
# target_feature="lzcnt"
# target_feature="pclmulqdq"
# target_feature="popcnt"
# target_feature="sse"
# target_feature="sse2"
# target_feature="sse3"
# target_feature="sse4.1"
# target_feature="sse4.2"
# target_feature="ssse3"
# target_feature="xsave"
# target_feature="xsaveopt"
# target_os="windows"
# target_pointer_width="64"
# target_vendor="pc"
# windows


# -C --target-feature=

def release_linux():
    ver = get_version_number()
    shell(f"set RUSTFLAGS=-Ctarget-feature={MODERN} && cargo b --release --features=fast --target x86_64-unknown-linux-musl  -C lto=fat")
    shell("ldd target/x86_64-unknown-linux-musl/release/odonata")
    shell(f"cp ./target/release/odonata.exe ./odonata-{ver}-linux-modern.exe")
    shell(f"set RUSTFLAGS=-Ctarget-feature={GENERIC} && cargo b --release --target x86_64-unknown-linux-musl")
    shell(f"cp ./target/release/odonata.exe ./odonata-{ver}-linux-generic.exe")

def release_mac():
    ver = get_version_number()
    shell("set RUSTFLAGS=-Ctarget-feature={MODERN} && cargo b --release -features=fast")
    shell(f"cp ./target/release/odonata.exe ./odonata-{ver}-darwin-modern.exe")

def release_windows():
    ver = get_version_number()
    shell(f'set RUSTFLAGS=-Ctarget-feature=+crt-static,{MODERN} && cargo b --release --features=fast -C lto=fat')
    shell(f"cp .\\target\\release\\odonata.exe .\\odonata-{ver}-windows-modern.exe")
    shell(f'set RUSTFLAGS=-Ctarget-feature=+crt-static,{GENERIC} && cargo b --release')
    shell(f"cp .\\target\\release\\odonata.exe .\\odonata-{ver}-windows-generic.exe")


def test_threading():
    shell("cargo t test_threading --release -- --ignored --nocapture")

def print_version():
    ver = get_version_number()
    print(f"Version number from Cargo.toml is {ver}\n")

def help():
    print("Build commands...")
    for cmd in commands.keys():
        print(f"  {cmd}")


commands = {
    "test_threading": test_threading,
    "release_linux": release_linux,
    "release_mac": release_mac,
    "release_windows": release_windows,
    "print_version": print_version,
    "help": help,

}


def main():
    for i, value in enumerate(sys.argv):
        # print(f"{value}\n")
        if value in commands:
            commands[value]()


if __name__ == "__main__":
    main()

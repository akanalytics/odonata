#!/usr/bin/env python3

import os
import sys
import platform
from typing import Optional
import subprocess


# See https://en.wikipedia.org/wiki/X86_Bit_manipulation_instruction_set


# target triple 
#
# eg x86_64-unknown-linux-gnu. 
#
# arch = x86_64, 
# vendor = unknown, sys = linux, 
# abi = gnu


#
# whats availabale
#
# rustc --print target-features
# rustc --print target-cpus
# rustc --print target-list
#

# show what you have
# rustc --print cfg
# rustc --print cfg -C target-cpu=native
#

#
# compiler spec file
#
# https://github.com/rust-lang/rust/tree/f2ea2f648e117013b0217f001088ae89e0f163ca/compiler/rustc_target/src/spec
#

# andys windows pc
# athlon64 860K
# https://browser.geekbench.com/processors/amd-athlon-x4-860k
#

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



# andys media server
# Intel Celeron 2955U (haswell)
# https://browser.geekbench.com/processors/intel-celeron-2955u
#
# Processor misdiagnosed as a haswell by llvm. Needed to use...
#  
# -Ctarget-cpu=sandybridge -Ctarget-feature=-avx,-xsave,-xsaveopt
#

# rustc --print  cfg -Ctarget-cpu=native
# debug_assertions
# target_arch="x86_64"
# target_endian="little"
# target_env="gnu"
# target_family="unix"
# target_feature="fxsr"
# target_feature="lzcnt"
# target_feature="pclmulqdq"
# target_feature="popcnt"
# target_feature="rdrand"
# target_feature="sse"
# target_feature="sse2"
# target_feature="sse3"
# target_feature="sse4.1"
# target_feature="sse4.2"
# target_feature="ssse3"
# target_os="linux"
# target_pointer_width="64"
# target_vendor="unknown"
# unix

# (venv) andy@kodi:~/code/odonata$ rustc --print  cfg
# debug_assertions
# target_arch="x86_64"
# target_endian="little"
# target_env="gnu"
# target_family="unix"
# target_feature="fxsr"
# target_feature="sse"
# target_feature="sse2"
# target_os="linux"
# target_pointer_width="64"
# target_vendor="unknown"
# unix

# this might be more typical but is still very slow if target-cpu=generic
# rust doesnt fully optimise for features unless a target cpu is specified
# MODERN = "+bmi,+popcnt,+lzcnt"

# modern based on andy's media server :-)
MODERN = "-avx,-xsave,-xsaveopt"
CPU="sandybridge"
GENERIC = ""

# clone the env so we do not change anything for rest of script. This is used by shell(...)
ENV = dict( os.environ )

def setenv(key: str, value: str):
    ENV[key] = value
    print(f">> {key} = {value}")


def shell(cmd: str):
    print(f">> {cmd}\n")
    return subprocess.run(cmd, shell=True, env=ENV)



def get_version_number() -> Optional[str]:
    with open("Cargo.toml", "r") as myfile:
        lines = myfile.readlines()
    for line in lines:
        if "version" in line:
            (_k,v) = line.split("=")
            v = v.strip().strip('"')
            return v
    return None


def release_linux():
    release_modern()
    release_generic()


def release_modern_old():
    ver = get_version_number()
    # shell("ldd target/x86_64-unknown-linux-musl/release/odonata")
    setenv("RUSTFLAGS", f"-Ctarget-feature={MODERN} -C target-cpu={CPU}")
    shell(f'cargo b --release --features=fast --target x86_64-unknown-linux-musl')
    shell(f"cp ./target/x86_64-unknown-linux-musl/release/odonata ./odonata-{ver}-linux-modern")
    shell(f"strip ./odonata-{ver}-linux-modern")

def release_modern():
    ver = get_version_number()
    # shell("ldd target/x86_64-unknown-linux-musl/release/odonata")
    setenv("RUSTFLAGS", f"-Ctarget-feature= -C target-cpu=x86-64-v3")
    shell(f'cargo b --release --features=fast --target x86_64-unknown-linux-musl')
    shell(f"cp ./target/x86_64-unknown-linux-musl/release/odonata ./odonata-{ver}-linux-modern")
    shell(f"strip ./odonata-{ver}-linux-modern")

def release_generic():
    ver = get_version_number()
    setenv("RUSTFLAGS", f"-Ctarget-feature={GENERIC} -C target-cpu=generic")
    shell(f'cargo b --release --target x86_64-unknown-linux-musl')
    shell(f"cp ./target/x86_64-unknown-linux-musl/release/odonata ./odonata-{ver}-linux-generic")
    shell(f"strip ./odonata-{ver}-linux-generic")

def release_native():
    ver = get_version_number()
    # shell("ldd target/x86_64-unknown-linux-musl/release/odonata")
    CPU="native"
    setenv("RUSTFLAGS", f"-Ctarget-feature={MODERN} -C target-cpu={CPU}")
    shell(f'cargo b --release --features=fast --target x86_64-unknown-linux-musl')
    shell(f"cp ./target/x86_64-unknown-linux-musl/release/odonata ./odonata-{ver}-linux-native")
    shell(f"strip ./odonata-{ver}-linux-native")

def release_mac():
    ver = get_version_number()
    setenv("RUSTFLAGS", f"-Ctarget-feature={MODERN} -C target-cpu={CPU}")
    shell("cargo b --release -features=fast")
    shell(f"cp ./target/release/odonata.exe ./odonata-{ver}-darwin-modern.exe")

def release_windows():
    CPU="nehalem"
    ver = get_version_number()
    setenv("RUSTFLAGS", f"-Ctarget-feature=+crt-static,{MODERN} -C target-cpu={CPU}")
    shell('cargo b --release --features=fast')
    shell(f"cp .\\target\\release\\odonata.exe .\\odonata-{ver}-windows-modern.exe")
    setenv("RUSTFLAGS", f"-Ctarget-feature=+crt-static,{GENERIC} -C target-cpu=generic")
    shell(f'cargo b --release')
    shell(f"cp .\\target\\release\\odonata.exe .\\odonata-{ver}-windows-generic.exe")


def test_threading():
    shell("cargo t test_threading --release -- --ignored --nocapture")

def print_version():
    ver = get_version_number()
    print(f"Version number from Cargo.toml is {ver}\n")

def setup():
    shell(f'rustup target add x86_64-unknown-linux-musl')

def help():
    print("Build commands...")
    for cmd in commands.keys():
        print(f"  {cmd}")


commands = {
    "test_threading": test_threading,
    "release_linux": release_linux,
    "release_modern_old": release_modern_old,
    "release_modern": release_modern,
    "release_generic": release_generic,
    "release_native": release_native,
    "release_mac": release_mac,
    "release_windows": release_windows,
    "print_version": print_version,
    "setup": setup,
    "help": help,

}


def main():
    for i, value in enumerate(sys.argv):
        # print(f"{value}\n")
        if value in commands:
            commands[value]()


if __name__ == "__main__":
    main()

# csbindgen — Generate C# native code bridge automatically or modern approaches to native code invocation from C#
[© Yoshifumi Kawai](https://neuecc.medium.com/csbindgen-generate-c-native-code-bridge-automatically-or-modern-approaches-to-native-code-78d9f9a616fb) · 14 min read · Mar 15, 2023

I have created and released a library that automatically generates C# DllImport code from Rust’s FFI to transparently connect native code and C#. This allows for native code to be smoothly called from C#.
- [Cysharp/csbindgen](https://github.com/Cysharp/csbindgen)
- [crates.io/crates/csbindgen](https://crates.io/crates/csbindgen)

First of all, it is important to consider that you should make an effort to optimize your code in C# rather than using native code. There are many reasons not to write native code. Among them, the most significant reason I want to avoid is the difficulty of building, especially cross-platform builds. In today’s world, the combination of platforms/architectures that must be targeted easily exceeds 10: win/linux/osx/iOS/Android and x86/x64/arm. In C#, the .NET runtime and Unity take care of this, but with native code, you have to handle it yourself.

However, there are still situations where you should use native code while primarily using C#:
- When you want to use something that only provides native APIs, such as Android NDK or .NET unmanaged hosting API
- When you want to use a native library written in C
- When you want to avoid using runtime libraries, for example, writing native network code in Unity to avoid .NET’s Socket (in Unity, the .NET runtime is old, making it difficult to achieve good performance)

The first choice for creating native code is, of course, C++, but C++ builds are extremely complex. That’s why I chose [Rust](https://www.rust-lang.org/). With libraries like the [cc crate](https://crates.io/crates/cc) and [cmake crate](https://crates.io/crates/cmake), C and C++ code can be naturally integrated into Rust’s build system, and automatic binding generation using [bindgen](https://github.com/rust-lang/rust-bindgen) is very stable. The development environment is well-equipped, and the command system is modern. Cross-platform builds are easy! It’s a great language.

However, integrating Rust code with C# requires an extra step. While there are tools like [SWIG](https://www.swig.org/), [ClangSharpPInvokeGenerator](https://github.com/dotnet/ClangSharp) and [CppSharp](https://github.com/mono/CppSharp) for automating DllImport, the idea of directly converting regular C++ code often results in incomplete or complex generated code, which is not ideal.

Csbindgen delegates the handling of complex C (C++) code to Rust’s bindgen. By having bindgen clean up the code into beautiful Rust and targeting only FFI-optimized Rust code for analysis, we ensure accuracy and simplicity of the generated code. When writing native code yourself, Rust warns you if you try to expose FFI-incompatible types, which inevitably results in clean and easy-to-generate code. Rust’s type system is also very organized, making it easier to map to C#. In recent years, C# has added features like nint, delegate*, and [CLong](https://learn.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.clong) (from .NET 6) that enable more natural interactions. Csbindgen leverages these latest language features to generate natural, high-performance binding code.

## Getting Started
To get started, simply add the build-time dependency to your config and insert the settings in `build.rs`, a pre-compile call (Rust's ability to write pre-build code and add build-time dependencies is excellent).

```toml
[build-dependencies]
csbindgen = "1.9.3"
```

```rust
// load `extern "C" fn` from lib.rs and generate DllImport["nativelib"] code to "NativeMethods.g.cs"
csbindgen::Builder::default()
    .input_extern_file("lib.rs")
    .csharp_dll_name("nativelib")
    .generate_csharp_file("../dotnet/NativeMethods.g.cs")
    .unwrap();R
```

For example, let’s take a simple function that takes x and y as inputs and returns an int.

```rust
#[no_mangle]
pub extern "C" fn my_add(x: i32, y: i32) -> i32 {
    x + y
}
```
The corresponding C# code generated would be:

```csharp
// NativeMethods.g.cs
using System;
using System.Runtime.InteropServices;

namespace CsBindgen
{
    internal static unsafe partial class NativeMethods
    {
        const string __DllName = "nativelib";

        [DllImport(__DllName, EntryPoint = "my_add", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        public static extern int my_add(int x, int y);
    }
}
```
It’s an intuitive and simple output. Simplicity is extremely important for an automatic generation tool. In addition to primitive types, the generated code supports most types that can be used in Rust’s FFI, such as structs, unions, enums, functions, and pointers.

Furthermore, by combining Rust’s bindgen and cc/cmake crates, you can easily integrate C libraries into C#. For example, the lz4 compression library can be brought into C# by adding the settings for bindgen and cc before generating with csbindgen:
```rust
// load lz4.h and output Rust bindgen to lz4.rs
bindgen::Builder::default()
    .header("c/lz4/lz4.h")
    .generate().unwrap()
    .write_to_file("lz4.rs").unwrap();

// load lz4.c and compile by rust cc(C Compiler)
cc::Build::new().file("lz4.c").compile("lz4");

// load bindgen output code and generate cs
csbindgen::Builder::default()
    .input_bindgen_file("lz4.rs")
    .rust_file_header("use super::lz4::*;")
    .csharp_entry_point_prefix("csbindgen_")
    .csharp_dll_name("liblz4")
    .generate_to_file("lz4_ffi.rs", "../dotnet/NativeMethods.lz4.g.cs")
    .unwrap();
```
With this, you can easily generate code that can be called from C#. Building is as simple as running `cargo build` in Rust, and the C code will be linked and included in the DLL.
```csharp
// NativeMethods.lz4.g.cs

using System;
using System.Runtime.InteropServices;

namespace CsBindgen
{
    internal static unsafe partial class NativeMethods
    {
        const string __DllName = "liblz4";

        [DllImport(__DllName, EntryPoint = "csbindgen_LZ4_compress_default", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        public static extern int LZ4_compress_default(byte* src, byte* dst, int srcSize, int dstCapacity);

        // snip...
    }
}
```

Thanks to Rust’s ecosystem, you can really easily integrate C libraries.

Csbindgen is designed with Unity in mind, so if you want to change the generated rules for situations like iOS’s IL2CPP where you only want to use `__Internal`:

```rust
#if UNITY_IOS && !UNITY_EDITOR
    const string __DllName = "__Internal";
#else
    const string __DllName = "nativelib";
#endif
```

These rule changes are included in the config.

## LibraryImport vs DllImport
Starting with .NET 7, a new source generator called [LibraryImport](https://learn.microsoft.com/en-us/dotnet/standard/native-interop/pinvoke-source-generation) has been added for invoking native code. It acts as a wrapper for DllImport, which automatically handles types that cannot be directly passed between native code and .NET (e.g., reference types like arrays and strings that exist on the C# heap). This automatic handling has caused some complications, performance issues, and problems when used with NativeAOT. When such types are passed, the generated C# code by LibraryImport absorbs them and passes them as byte* to DllImport.

In other words, if we avoid generating types that cannot be directly passed between native code and .NET, there will be no problem with using DllImport. Therefore, csbindgen has chosen to generate code for DllImport.

The complex features of DllImport were designed to facilitate easy calling of Win32 APIs by providing numerous implicit automatic conversions. While this can be understood from a historical perspective, today’s languages are not just for Windows, and support for calling Win32 APIs now exists in the form of the [CsWin32](https://github.com/microsoft/CsWin32) source generator.

In the modern context, there is no need to be burdened by the old design of DllImport. We should not pass reference types or use [In] and [Out], and we don’t need to design with these conversions in mind. In fact, .NET 7 introduced the [DisableRuntimeMarshallingAttribute](https://learn.microsoft.com/en-us/dotnet/api/system.runtime.compilerservices.disableruntimemarshallingattribute?view=net-7.0), which throws an error when using such DllImport features.

As for pointers, they are not as taboo as they once were. Communication with native code is inherently unsafe, and it is relatively easy to convert them to more user-friendly types like Span. Instead of partially concealing pointers, we should keep them as pointers at the DllImport layer. Making it more user-friendly in C# can be properly done outside of DllImport. This is the modern design philosophy that I believe we should follow.

## Exchanging Callbacks Between Languages
Let’s try exchanging callbacks between C# and Rust. First, let’s write the Rust side like this:

```rust
#[no_mangle]
pub extern "C" fn csharp_to_rust(cb: extern "C" fn(x: i32, y: i32) -> i32) {
    let sum = cb(10, 20); // invoke C# method
    println!("{sum}");
}

#[no_mangle]
pub extern "C" fn rust_to_csharp() -> extern fn(x: i32, y: i32) -> i32 {
    sum // return rust method
}

extern "C" fn sum(x:i32, y:i32) -> i32 {
    x + y
}
```















Text by Yoshifumi Kawai extracted from https://neuecc.medium.com/csbindgen-generate-c-native-code-bridge-automatically-or-modern-approaches-to-native-code-78d9f9a616fb (15th of August 2025)

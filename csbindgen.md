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





Text by Yoshifumi Kawai extracted from https://neuecc.medium.com/csbindgen-generate-c-native-code-bridge-automatically-or-modern-approaches-to-native-code-78d9f9a616fb (15th of August 2025)

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

Upon receiving a C# method, the simple method will either just read and display it using println or pass an addition function back to C#. The generated code will look like this:

```csharp
[DllImport(__DllName, EntryPoint = "csharp_to_rust", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
public static extern void csharp_to_rust(delegate* unmanaged[Cdecl]<int, int, int> cb);

[DllImport(__DllName, EntryPoint = "rust_to_csharp", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
public static extern delegate* unmanaged[Cdecl]<int, int, int> rust_to_csharp();
```

`delegate* unmanaged[Cdecl]<int, int, int>` might be an unfamiliar definition, but it is a true function pointer added in C# 9.0. Although manually writing the definition is a bit complicated, it is automatically generated, so there is no need to write it by hand. The usability is quite good and can be treated like a regular static method.

```csharp
// C# to Native, require UnmanagedCallersOnly
[UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
static int Sum(int x, int y) => x + y;

// pass function pointer by `&`
NativeMethods.csharp_to_rust(&Sum);

// receive delegate* from Rust
var f = NativeMethods.rust_to_csharp();

// received function pointer can invoke naturally
var v = f(20, 30);
Console.WriteLine(v); // 50
```

If you want to pass around state, you can prepare code that takes a context (void*) as the first argument.

By the way, Unity supports C# 9.0 and function pointers can be used, but [extensible calling conventions for unmanaged function pointers is not supported](https://docs.unity3d.com/Manual/CSharpCompiler.html). UnmanagedCallersOnlyAttribute is also missing. In particular, it does not work at all with IL2CPP, so special measures are needed. By setting the csharp_use_function_pointer(false) option in csbindgen, it will output code using the traditional delegate.

```csharp
// csharp_use_function_pointer(false) generates dedicated delegate
[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
public delegate int csharp_to_rust_cb_delegate(int x, int y);

[DllImport(__DllName, EntryPoint = "csharp_to_rust", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
public static extern void csharp_to_rust(csharp_to_rust_cb_delegate cb);

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
public delegate int rust_to_csharp_return_delegate(int x, int y);

[DllImport(__DllName, EntryPoint = "rust_to_csharp", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
public static extern rust_to_csharp_return_delegate rust_to_csharp();

// require MonoPInvokeCallback(setup delegate type by typeof)
[MonoPInvokeCallback(typeof(NativeMethods.csharp_to_rust_cb_delegate))]
static int Sum(int x, int y) => x + y;

// pass directly
NativeMethods.csharp_to_rust(Method);

// received function pointer(delegate) is same as .NET
var f = NativeMethods.rust_to_csharp();
var v = f(20, 30);
Console.WriteLine(v); // 50
```

Csbindgen also outputs dedicated delegates simultaneously, making the definition much easier. The only difference between .NET and Unity that needs to be considered is the attribute, and there should be almost no problem.


```rust
#[no_mangle]
pub unsafe extern "C" fn return_tuple() -> MyTuple {
    MyTuple { is_foo: true, bar: 9999 }
}

#[repr(C)]
pub struct MyTuple {
    pub is_foo: bool,
    pub bar: i32,
}
```

If you want to keep the returned Struct’s state longer by returning it as a pointer, a little ingenuity is required in Rust.

```rust
#[no_mangle]
pub extern "C" fn create_context() -> *mut Context {
    let ctx = Box::new(Context { foo: true });
    Box::into_raw(ctx)
}

#[no_mangle]
pub extern "C" fn delete_context(context: *mut Context) {
    unsafe { Box::from_raw(context) };
}

#[repr(C)]
pub struct Context {
    pub foo: bool,
    pub bar: i32,
    pub baz: u64
}
```

```csharp
// in C#側, receiveContext*
var context = NativeMethods.create_context();

// do something

// finally, call free explicitly
NativeMethods.delete_context(context);
```

Allocate data on the heap with Box::new and remove it from Rust’s memory management with Box::into_raw. Rust usually returns memory immediately when it goes out of scope, but since the lifetime is transferred to C# which is outside of Rust’s management, it is straightforward to remove it unsafely from Rust’s management. To release memory allocated on the Rust side, return it to Rust’s management with Box::from_raw. Then, when the scope is exited, it will perform the usual operation of returning memory, and the return will be completed.

This is not a matter of difficulty because of Rust; in C#, if you want to manage pointers outside of a fixed scope, you need to use GCHandle.Alloc(obj, GCHandleType.Pinned) and manually manage it unsafely, so it’s the same story.

There is a style of creating a dedicated SafeHandle in C# for managing such contexts and wrapping it, but I don’t think it’s necessary to go that far. After all, you’re doing something unsafe by crossing boundaries, so you might as well take responsibility until the end.

Csbindgen will try to generate something similar on the C# side when a struct is specified as a return value, but I think there may be cases where you want to use it only within Rust and not expose the contents to the C# side, or you can’t expose it because it contains references (Box) or something. In that case, please return a c_void. Alternatively, you can use a dedicated empty struct pointer, which is better because the handle represented by the pointer is distinguished by the type.

```rust
#[no_mangle]
pub extern "C" fn create_counter_context() -> *mut c_void {
    let ctx = Box::new(CounterContext {
        set: HashSet::new(),
    });
    Box::into_raw(ctx) as *mut c_void // return void
}

#[no_mangle]
pub unsafe extern "C" fn insert_counter_context(context: *mut c_void, value: i32) {
    let mut counter = Box::from_raw(context as *mut CounterContext); // type convert by as
    counter.set.insert(value);
    Box::into_raw(counter); // require into_raw if continue to use context
}

#[no_mangle]
pub unsafe extern "C" fn delete_counter_context(context: *mut c_void) {
    let counter = Box::from_raw(context as *mut CounterContext);
    for value in counter.set.iter() {
        println!("counter value: {}", value)
    }
}

// not expose to C#
pub struct CounterContext {
    pub set: HashSet<i32>,
}
```

```csharp
// in C#, receive ctx = void*
var ctx = NativeMethods.create_counter_context();

NativeMethods.insert_counter_context(ctx, 10);
NativeMethods.insert_counter_context(ctx, 20);

NativeMethods.delete_counter_context(ctx);
```

## Marshaling Strings and Arrays

Strings and arrays have different structures in C# and Rust, so they cannot be directly exchanged. You can only exchange pointers and lengths, which are Spans in C#. If you only need to process Spans, it’s zero-copy. However, if you want to convert to a string or an array, you will need to allocate memory on both the C# and Rust sides. This is a drawback of introducing native code, as pure C# is more flexible (or may be advantageous in terms of performance). Anyway, the basic idea is to use Spans. You should not accept strings or arrays in DllImport; instead, manage the allocations explicitly without relying on automatic conversions.

Now, let’s talk about strings. There are three types of strings to be exchanged in such cases: UTF8, UTF16, and null-terminated strings. UTF8 corresponds to Rust’s strings (Rust’s String is Vec<u8>), C#’s strings are UTF16, and C libraries may return null-terminated strings.

For this example, we will explicitly return a null-terminated string in Rust.

```rust
#[no_mangle]
pub extern "C" fn alloc_c_string() -> *mut c_char {
    let str = CString::new("foo bar baz").unwrap();
    str.into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn free_c_string(str: *mut c_char) {
    unsafe { CString::from_raw(str) };
}
```

```csharp
// null-terminated `byte*` or sbyte* can materialize by new String()
var cString = NativeMethods.alloc_c_string();
var str = new String((sbyte*)cString);
NativeMethods.free_c_string(cString);
```

In C#, you can create a string by passing a pointer (sbyte*) to the new String, which will find the null terminator and create a string for you. In this case, the pointer is memory allocated in Rust, so once you’ve copied it onto the C# heap (created a new String), you should return it immediately.

Allocating UTF8, byte[], or int[] arrays in Rust and passing them to C# is a bit more complicated. When passing an array-like object (Vec<T>) from Rust to C#, it’s okay to pass a pointer and length, but this alone is not enough for deallocating memory. The actual Vec<T> consists of a pointer, length, and capacity, so you need to pass these three pieces of information. Processing these three pieces of information every time can be cumbersome, as there are tasks such as removing and returning Rust-like memory management.

To handle this, let’s prepare a slightly longer utility, as shown below. The original code for this utility comes from Mozilla, the (former) developer of Rust.

```rust
#[repr(C)]
pub struct ByteBuffer {
    ptr: *mut u8,
    length: i32,
    capacity: i32,
}

impl ByteBuffer {
    pub fn len(&self) -> usize {
        self.length.try_into().expect("buffer length negative or overflowed")
    }

    pub fn from_vec(bytes: Vec<u8>) -> Self {
        let length = i32::try_from(bytes.len()).expect("buffer length cannot fit into a i32.");
        let capacity = i32::try_from(bytes.capacity()).expect("buffer capacity cannot fit into a i32.");

        // keep memory until call delete
        let mut v = std::mem::ManuallyDrop::new(bytes);

        Self {
            ptr: v.as_mut_ptr(),
            length,
            capacity,
        }
    }

    pub fn from_vec_struct<T: Sized>(bytes: Vec<T>) -> Self {
        let element_size = std::mem::size_of::<T>() as i32;

        let length = (bytes.len() as i32) * element_size;
        let capacity = (bytes.capacity() as i32) * element_size;

        let mut v = std::mem::ManuallyDrop::new(bytes);

        Self {
            ptr: v.as_mut_ptr() as *mut u8,
            length,
            capacity,
        }
    }

    pub fn destroy_into_vec(self) -> Vec<u8> {
        if self.ptr.is_null() {
            vec![]
        } else {
            let capacity: usize = self.capacity.try_into().expect("buffer capacity negative or overflowed");
            let length: usize = self.length.try_into().expect("buffer length negative or overflowed");

            unsafe { Vec::from_raw_parts(self.ptr, length, capacity) }
        }
    }

    pub fn destroy_into_vec_struct<T: Sized>(self) -> Vec<T> {
        if self.ptr.is_null() {
            vec![]
        } else {
            let element_size = std::mem::size_of::<T>() as i32;
            let length = (self.length * element_size) as usize;
            let capacity = (self.capacity * element_size) as usize;

            unsafe { Vec::from_raw_parts(self.ptr as *mut T, length, capacity) }
        }
    }

    pub fn destroy(self) {
        drop(self.destroy_into_vec());
    }
}
```

This utility works like the Vec version of Box::into_raw/from_raw, removing memory management when calling from_vec and returning memory management to the caller when calling destroy_into_vec (it will be destroyed when the scope is exited if nothing is done). This definition is also generated on the C# side (by csbindgen), so you can add methods to it.

```csharp
// C# side span utility
partial struct ByteBuffer
{
    public unsafe Span<byte> AsSpan()
    {
        return new Span<byte>(ptr, length);
    }

    public unsafe Span<T> AsSpan<T>()
    {
        return MemoryMarshal.CreateSpan(ref Unsafe.AsRef<T>(ptr), length / Unsafe.SizeOf<T>());
    }
}
```

Now, you can instantly convert what you’ve received as ByteBuffer* to Span! Let’s take a look at examples of regular strings, byte[], and int[] in Rust.

```rust
#[no_mangle]
pub extern "C" fn alloc_u8_string() -> *mut ByteBuffer {
    let str = format!("foo bar baz");
    let buf = ByteBuffer::from_vec(str.into_bytes());
    Box::into_raw(Box::new(buf))
}

#[no_mangle]
pub unsafe extern "C" fn free_u8_string(buffer: *mut ByteBuffer) {
    let buf = Box::from_raw(buffer);
    // drop inner buffer, if you need String, use String::from_utf8_unchecked(buf.destroy_into_vec()) instead.
    buf.destroy();
}

#[no_mangle]
pub extern "C" fn alloc_u8_buffer() -> *mut ByteBuffer {
    let vec: Vec<u8> = vec![1, 10, 100];
    let buf = ByteBuffer::from_vec(vec);
    Box::into_raw(Box::new(buf))
}

#[no_mangle]
pub unsafe extern "C" fn free_u8_buffer(buffer: *mut ByteBuffer) {
    let buf = Box::from_raw(buffer);
    // drop inner buffer, if you need Vec<u8>, use buf.destroy_into_vec() instead.
    buf.destroy();
}

#[no_mangle]
pub extern "C" fn alloc_i32_buffer() -> *mut ByteBuffer {
    let vec: Vec<i32> = vec![1, 10, 100, 1000, 10000];
    let buf = ByteBuffer::from_vec_struct(vec);
    Box::into_raw(Box::new(buf))
}

#[no_mangle]
pub unsafe extern "C" fn free_i32_buffer(buffer: *mut ByteBuffer) {
    let buf = Box::from_raw(buffer);
    // drop inner buffer, if you need Vec<i32>, use buf.destroy_into_vec_struct::<i32>() instead.
    buf.destroy();
}
```

It can be confusing to have nested management, such as the need to remove the management of ByteBuffer itself (into_raw) and the need to destroy or into_vec the contents of the ByteBuffer after returning it with from_raw. There is room for improvement in the cleanup side of the process by implementing the Drop trait.

On the C# side, you can simply use AsSpan and use it as you like.

```csharp
var u8String = NativeMethods.alloc_u8_string();
var u8Buffer = NativeMethods.alloc_u8_buffer();
var i32Buffer = NativeMethods.alloc_i32_buffer();
try
{
    var str = Encoding.UTF8.GetString(u8String->AsSpan());
    Console.WriteLine(str);

    Console.WriteLine("----");

    var buffer = u8Buffer->AsSpan();
    foreach (var item in buffer)
    {
        Console.WriteLine(item);
    }

    Console.WriteLine("----");

    var i32Span = i32Buffer->AsSpan<int>();
    foreach (var item in i32Span)
    {
        Console.WriteLine(item);
    }
}
finally
{
    NativeMethods.free_u8_string(u8String);
    NativeMethods.free_u8_buffer(u8Buffer);
    NativeMethods.free_i32_buffer(i32Buffer);
}
```

Let’s remain faithful to the basic principle that memory allocated in Rust should be released in Rust. In this example, you might want the memory to be released automatically once it’s been processed on the C# side. However, there are cases where you want to keep the memory for a longer lifespan, so let’s release it manually. Implicit allocations are the number one enemy of performance.

Finally, here’s an example of using memory allocated in C# on the Rust side.

```rust
#[no_mangle]
pub unsafe extern "C" fn csharp_to_rust_string(utf16_str: *const u16, utf16_len: i32) {
    let slice = std::slice::from_raw_parts(utf16_str, utf16_len as usize);
    let str = String::from_utf16(slice).unwrap();
    println!("{}", str);
}

#[no_mangle]
pub unsafe extern "C" fn csharp_to_rust_utf8(utf8_str: *const u8, utf8_len: i32) {
    let slice = std::slice::from_raw_parts(utf8_str, utf8_len as usize);
    let str = String::from_utf8_unchecked(slice.to_vec());
    println!("{}", str);
}


#[no_mangle]
pub unsafe extern "C" fn csharp_to_rust_bytes(bytes: *const u8, len: i32) {
    let slice = std::slice::from_raw_parts(bytes, len as usize);
    let vec = slice.to_vec();
    println!("{:?}", vec);
}
```

```csharp
var str = "foobarbaz:あいうえお"; // JPN(Unicode)
fixed (char* p = str)
{
    NativeMethods.csharp_to_rust_string((ushort*)p, str.Length);
}

var str2 = Encoding.UTF8.GetBytes("あいうえお:foobarbaz");
fixed (byte* p = str2)
{
    NativeMethods.csharp_to_rust_utf8(p, str2.Length);
}

var bytes = new byte[] { 1, 10, 100, 255 };
fixed (byte* p = bytes)
{
    NativeMethods.csharp_to_rust_bytes(p, bytes.Length);
}
```

You create a Slice using std::slice::from_raw_parts and then process it as needed. If you want to maintain a longer lifespan beyond a single function, copying (creating a String, Vec, etc.) is essential. Just as it is important to release memory allocated in Rust on the Rust side, it is important to release memory allocated in C# on the C# side. In the case of C#, if you don’t have a reference after exiting the fixed scope, the GC will eventually handle it.

If you want to maintain a longer lifespan in C# beyond the fixed scope, you can use GCHandle.Alloc(obj, GCHandleType.Pinned) to carry it around.

## Conclusion

The [csbindgen]() ReadMe introduces many more conversion patterns, so be sure to check it out as well.

Bringing in C libraries has become overwhelmingly easier, which has changed my way of thinking a bit. Until now, I was more of a Pure C# implementation purist, but now I’ve learned to think about clever divisions and distinctions in usage. And as using C libraries becomes more flexible, it’s another step towards realizing [Cysharp](http://cysharp.com/)‘s mission of “unlocking the possibilities of C#”.

We have plans to provide several C# libraries utilizing csbindgen soon! However, before that, I’d be delighted if you could give csbindgen a try, even if you’ve never used Rust before.


















Text by Yoshifumi Kawai extracted from https://neuecc.medium.com/csbindgen-generate-c-native-code-bridge-automatically-or-modern-approaches-to-native-code-78d9f9a616fb (15th of August 2025)

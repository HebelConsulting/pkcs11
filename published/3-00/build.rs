// using bindgen, generate binding code
bindgen::Builder::default()
    .header("pkcs11.h")
    .generate().unwrap()
    .write_to_file("generated/pkcs11.rs").unwrap();

// using cc, build and link c code
cc::Build::new().file("generated/pkcs11.c").compile("generated/pkcs11");

// csbindgen code, generate both rust ffi and C# dll import
csbindgen::Builder::default()
    .input_bindgen_file("generated/pkcs11.rs")            // read from bindgen generated code
    .rust_file_header("use super::pkcs11::*;")     // import bindgen generated modules(struct/method)
    .csharp_entry_point_prefix("csbindgen_") // adjust same signature of rust method and C# EntryPoint
    .csharp_dll_name("libpkcs11")
    .generate_to_file("generated/pkcs11_ffi.rs", "generated/NativeMethods.pkcs11.g.cs")
    .unwrap();
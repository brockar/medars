fn main() {
    // Link to exiv2 for rexiv2
    println!("cargo:rustc-link-lib=dylib=exiv2");
}

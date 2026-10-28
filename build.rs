fn main() {
    println!("cargo::rerun-if-changed=src/robust/fec.c");
    // cc::Build::new()
    //     .file("src/robust/fec.c")
    //     .flag("-fPIC")
    //     .compile("janusfec");
}

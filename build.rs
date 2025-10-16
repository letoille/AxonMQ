fn main() {
    #[cfg(target_os = "macos")]
    {
        tonic_prost_build::configure()
            .build_server(true)
            //.build_client(true)
            .out_dir("src/service/sparkplug_b")
            .compile_protos(&["sparkplug_b.proto"], &["."])
            .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
    }
}

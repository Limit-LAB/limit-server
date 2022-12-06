fn main() {
    // auth.proto
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile(
            &[
                "../idl/auth.proto",
                "../idl/event.proto",
                "../idl/event.types.proto",
                "../idl/subs.proto",
                "../idl/subs.types.proto",
                "../idl/utils.proto",
            ],
            &["../idl"],
        )
        .unwrap();
}

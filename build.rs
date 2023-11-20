fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_include_paths(vec!["ui".into(), "ui/img".into()])
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/main.slint", config).unwrap();
}

use std::path::Path;

use cc::Build;
use rustc_version::Version;

fn main() {
    let docs_rs = std::env::var_os("DOCS_RS");

    let rust_version = rustc_version::version_meta().expect("unable to get rustc version");

    let all_ffmpeg_features = ["channel_layout_v2", "codec_params_side_data", "stream_side_data"];
    // force enable all features and skip the weird build dependency that breaks cross compilation
    // (can't link to windows ffmpeg when compiling on linux)
    let available_ffmpeg_features = all_ffmpeg_features;

    // Avoid cfg-check warnings on Rust >= 1.77.
    if version_gte(&rust_version.semver, 1, 77) {
        for feature in &all_ffmpeg_features {
            println!("cargo::rustc-check-cfg=cfg({feature})");
        }
    }

    for feature in &available_ffmpeg_features {
        println!("cargo:rustc-cfg={feature}");
    }

    // skip building native resources during docs.rs builds
    if docs_rs.is_some() {
        return;
    }

    let src_dir = Path::new("src");

    let src_format_dir = src_dir.join("format");
    let src_codec_dir = src_dir.join("codec");
    let src_codec_audio_dir = src_codec_dir.join("audio");
    let src_codec_video_dir = src_codec_dir.join("video");

    println!("cargo:rerun-if-changed={}", src_dir.display());

    let mut build = Build::new();

    build.include(src_dir);

    for dir in ac_ffmpeg_build::ffmpeg_include_dirs(true) {
        build.include(dir);
    }

    for feature in &available_ffmpeg_features {
        build.define(&format!("FFW_FEATURE_{}", feature.to_uppercase()), None);
    }

    build
        .file(src_dir.join("error.c"))
        .file(src_dir.join("logger.c"))
        .file(src_dir.join("packet.c"))
        .file(src_dir.join("time.c"))
        .file(src_format_dir.join("demuxer.c"))
        .file(src_format_dir.join("io.c"))
        .file(src_format_dir.join("muxer.c"))
        .file(src_format_dir.join("stream.c"))
        .file(src_codec_dir.join("bsf.c"))
        .file(src_codec_dir.join("mod.c"))
        .file(src_codec_dir.join("frame.c"))
        .file(src_codec_audio_dir.join("resampler.c"))
        .file(src_codec_video_dir.join("scaler.c"));

    if cfg!(feature = "filters") {
        build.file(src_codec_video_dir.join("filter.c"));
    }

    build.compile("ffwrapper");

    for dir in ac_ffmpeg_build::ffmpeg_lib_dirs(true) {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }

    let ffmpeg_link_mode = link_mode();

    if cfg!(feature = "filters") {
        link("avfilter", ffmpeg_link_mode);
    }

    link("avcodec", ffmpeg_link_mode);
    link("avformat", ffmpeg_link_mode);
    link("avutil", ffmpeg_link_mode);
    link("swresample", ffmpeg_link_mode);
    link("swscale", ffmpeg_link_mode);
}

/// Get the FFmpeg link mode.
fn link_mode() -> &'static str {
    std::env::var("FFMPEG_STATIC")
        .map(|v| match v.as_str() {
            "0" => "dylib",
            _ => "static",
        })
        .unwrap_or("dylib")
}

/// Link a given library.
fn link(lib: &str, mode: &str) {
    println!("cargo:rustc-link-lib={}={}", mode, lib);
}

fn version_gte(version: &Version, major: u64, minor: u64) -> bool {
    version.major > major || (version.major == major && version.minor >= minor)
}

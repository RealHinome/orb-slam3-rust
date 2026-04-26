use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let orb_slam_path = format!("{}/3rdparty/ORB_SLAM3", manifest_dir);
    let prebuilt_subdir = format!("{}-{}", target_os, target_arch);
    let prebuilt_path = PathBuf::from(&manifest_dir)
        .join("prebuilt")
        .join(&prebuilt_subdir);

    if target_os == "macos" {
        unsafe {
            env::set_var("MACOSX_DEPLOYMENT_TARGET", "11.0");
        };
    }

    let g2o_build_path = Path::new(&manifest_dir).join("3rdparty/ORB_SLAM3/Thirdparty/g2o/build");
    let g2o_config_h = g2o_build_path.join("config.h");

    if !g2o_config_h.exists() {
        println!("cargo:warning=G2O config.h not found, generating with CMake...");
        let _ = std::fs::create_dir_all(&g2o_build_path);

        Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POLICY_VERSION_MINIMUM=3.5")
            .current_dir(&g2o_build_path)
            .status()
            .expect("Failed to build g2O with CMake");
    }

    let mut extra_clang_args = vec!["-std=c++14".to_string()];

    if target_os == "macos" {
        let sdk_path = Command::new("xcrun")
            .args(["--show-sdk-path"])
            .output()
            .expect("Failed to get macOS SDK path");

        let sdk_path_str = std::str::from_utf8(&sdk_path.stdout).unwrap().trim();
        extra_clang_args.push(format!("-isysroot{}", sdk_path_str));

        extra_clang_args.push("-I/opt/homebrew/include".to_string());
        extra_clang_args.push("-I/opt/homebrew/opt/opencv/include/opencv4".to_string());
        extra_clang_args.push("-I/opt/homebrew/opt/openssl/include".to_string());
    }

    let include_path = PathBuf::from("cpp");
    let mut b = autocxx_build::Builder::new("src/lib.rs", [&include_path])
        .extra_clang_args(
            &extra_clang_args
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        )
        .build()
        .expect("Failed to generate C++ bindings");

    let build = b.flag_if_supported("-std=c++14");
    build
        .file("cpp/slam_wrapper.cpp")
        .include(&orb_slam_path)
        .include(format!("{}/include", orb_slam_path))
        .include(format!("{}/include/CameraModels", orb_slam_path))
        .include(format!("{}/Thirdparty/Sophus", orb_slam_path))
        .include(format!("{}/compat", manifest_dir));

    if target_os == "macos" {
        build
            .include("/opt/homebrew/include")
            .include("/opt/homebrew/include/eigen3")
            .include("/opt/homebrew/opt/opencv/include/opencv4")
            .include("/opt/homebrew/opt/openssl/include");
    } else {
        build
            .include("/usr/include/eigen3")
            .include("/usr/include/opencv4");
    }

    build.compile("orb_slam_wrapper");

    if prebuilt_path.exists() {
        println!("cargo:rustc-link-search=native={}", prebuilt_path.display());
    } else {
        println!("cargo:rustc-link-search=native={}/lib", orb_slam_path);
        println!(
            "cargo:rustc-link-search=native={}/Thirdparty/g2o/lib",
            orb_slam_path
        );
        println!(
            "cargo:rustc-link-search=native={}/Thirdparty/DBoW2/lib",
            orb_slam_path
        );
    }

    println!("cargo:rustc-link-lib=static=ORB_SLAM3");
    println!("cargo:rustc-link-lib=static=g2o");

    let dbow_static = prebuilt_path.join("libDBoW2.a");
    let dbow_submodule_static = Path::new(&orb_slam_path).join("Thirdparty/DBoW2/lib/libDBoW2.a");

    if dbow_static.exists() || dbow_submodule_static.exists() {
        println!("cargo:rustc-link-lib=static=DBoW2");
    } else {
        println!("cargo:rustc-link-lib=dylib=DBoW2");
    }

    if target_os == "macos" {
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
        println!("cargo:rustc-link-search=native=/opt/homebrew/opt/openssl/lib");

        println!("cargo:rustc-link-lib=dylib=opencv_core");
        println!("cargo:rustc-link-lib=dylib=opencv_imgproc");
        println!("cargo:rustc-link-lib=dylib=opencv_videoio");
        println!("cargo:rustc-link-lib=dylib=opencv_features2d");
        println!("cargo:rustc-link-lib=dylib=opencv_calib3d");
        println!("cargo:rustc-link-lib=dylib=boost_serialization");
        println!("cargo:rustc-link-lib=dylib=crypto");
        println!("cargo:rustc-link-lib=dylib=c++");

        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=framework=OpenCL");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
    } else if target_os == "linux" {
        println!("cargo:rustc-link-search=native=/usr/lib/aarch64-linux-gnu");
        println!("cargo:rustc-link-search=native=/usr/local/lib");

        println!("cargo:rustc-link-lib=dylib=opencv_core");
        println!("cargo:rustc-link-lib=dylib=opencv_imgproc");
        println!("cargo:rustc-link-lib=dylib=opencv_videoio");
        println!("cargo:rustc-link-lib=dylib=opencv_features2d");
        println!("cargo:rustc-link-lib=dylib=opencv_calib3d");
        println!("cargo:rustc-link-lib=dylib=boost_serialization");
        println!("cargo:rustc-link-lib=dylib=crypto");
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:rerun-if-changed=cpp/slam_wrapper.h");
    println!("cargo:rerun-if-changed=cpp/slam_wrapper.cpp");
    println!("cargo:rerun-if-changed=src/lib.rs");
}

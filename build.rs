use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = Path::new(&manifest_dir);

    let orb_slam_path = manifest_path.join("3rdparty/ORB_SLAM3");
    let compat_path = manifest_path.join("compat");

    let prebuilt_subdir = format!("{}-{}", target_os, target_arch);
    let prebuilt_path = manifest_path.join("prebuilt").join(&prebuilt_subdir);

    let mut brew_prefix = "/opt/homebrew".to_string();
    if target_os == "macos" {
        if let Ok(output) = Command::new("brew").args(["--prefix"]).output() {
            brew_prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        unsafe {
            env::set_var("MACOSX_DEPLOYMENT_TARGET", "11.0");
        }
    }

    let g2o_source = orb_slam_path.join("Thirdparty/g2o");
    let g2o_config_h = g2o_source.join("config.h");

    if !g2o_config_h.exists() {
        println!(
            "cargo:warning=Generating static g2o config.h at {}",
            g2o_config_h.display()
        );
        let config_content = r#"
#ifndef G2O_CONFIG_H
#define G2O_CONFIG_H

/* #undef G2O_OPENMP */
/* #undef G2O_SHARED_LIBS */

#ifdef EIGEN_DEFAULT_TO_ROW_MAJOR
#  error "g2o requires column major Eigen matrices (see http://eigen.tuxfamily.org/bz/show_bug.cgi?id=422)"
#endif

#endif
"#;
        std::fs::write(&g2o_config_h, config_content).expect("Failed to write g2o config.h");
    }

    if !prebuilt_path.exists() {
        let dst = cmake::Config::new(&g2o_source)
            .define("CMAKE_CXX_STANDARD", "14")
            .define("CMAKE_CXX_STANDARD_REQUIRED", "ON")
            .define("G2O_USE_OPENMP", "OFF")
            .cxxflag(format!("-I{}", compat_path.display()))
            .no_build_target(true)
            .build();

        println!("cargo:rustc-link-search=native={}/build", dst.display());
    }

    let mut extra_clang_args = vec!["-std=c++14".to_string()];

    if target_os == "macos" {
        if let Ok(output) = Command::new("xcrun").args(["--show-sdk-path"]).output() {
            let sdk_path = std::str::from_utf8(&output.stdout).unwrap().trim();
            extra_clang_args.push(format!("-isysroot{}", sdk_path));
        }

        extra_clang_args.push(format!("-I{}/include", brew_prefix));
        extra_clang_args.push(format!("-I{}/opt/opencv/include/opencv4", brew_prefix));
        extra_clang_args.push(format!("-I{}/opt/openssl/include", brew_prefix));
    }

    let include_path = manifest_path.join("cpp");
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
        .flag_if_supported("-Wno-deprecated-declarations")
        .flag_if_supported("-Wno-nonportable-include-path")
        .file("cpp/slam_wrapper.cpp")
        .include(&orb_slam_path)
        .include(orb_slam_path.join("include"))
        .include(orb_slam_path.join("include/CameraModels"))
        .include(orb_slam_path.join("Thirdparty/Sophus"))
        .include(&compat_path);

    if target_os == "macos" {
        build
            .include(format!("{}/include", brew_prefix))
            .include(format!("{}/include/eigen3", brew_prefix))
            .include(format!("{}/opt/opencv/include/opencv4", brew_prefix))
            .include(format!("{}/opt/openssl/include", brew_prefix));
    } else {
        build
            .include("/usr/include/eigen3")
            .include("/usr/include/opencv4");
    }

    build.compile("orb_slam_wrapper");

    if prebuilt_path.exists() {
        println!("cargo:rustc-link-search=native={}", prebuilt_path.display());
    } else {
        println!(
            "cargo:rustc-link-search=native={}/lib",
            orb_slam_path.display()
        );
        println!(
            "cargo:rustc-link-search=native={}/Thirdparty/g2o/lib",
            orb_slam_path.display()
        );
        println!(
            "cargo:rustc-link-search=native={}/Thirdparty/DBoW2/lib",
            orb_slam_path.display()
        );
    }

    println!("cargo:rustc-link-lib=static=ORB_SLAM3");
    println!("cargo:rustc-link-lib=static=g2o");

    let dbow_static = prebuilt_path.join("libDBoW2.a");
    let dbow_submodule_static = orb_slam_path.join("Thirdparty/DBoW2/lib/libDBoW2.a");

    if dbow_static.exists() || dbow_submodule_static.exists() {
        println!("cargo:rustc-link-lib=static=DBoW2");
    } else {
        println!("cargo:rustc-link-lib=dylib=DBoW2");
    }

    if target_os == "macos" {
        println!("cargo:rustc-link-search=native={}/lib", brew_prefix);
        println!(
            "cargo:rustc-link-search=native={}/opt/openssl/lib",
            brew_prefix
        );

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

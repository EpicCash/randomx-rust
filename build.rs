extern crate bindgen;
extern crate cmake;
extern crate filetime;

use filetime::FileTime;

use std::env;
use std::fs;

pub fn fail_on_empty_directory(name: &str) {
	if fs::read_dir(name).unwrap().count() == 0 {
		println!(
			"The `{}` directory is empty. Did you forget to pull the submodules?",
			name
		);
		println!("Try `git submodule update --init --recursive`");
		panic!();
	}
}

fn generate_bindings(out_dir: &str) {
	let target = env::var("TARGET").unwrap();

	// For iOS simulator, use a modified target for bindgen
	let mut builder = bindgen::Builder::default()
		.header("randomx/src/randomx.h");

	// iOS simulator targets need special handling- use the device target for bindgen
	if target.contains("ios-sim") {
		println!("cargo:warning=Using iOS device target for bindgen on simulator");

		builder = builder.clang_arg("--target=arm64-apple-ios");
	}

	let bindings = builder
		.generate()
		.expect("Unable to generate bindings");

	bindings
		.write_to_file(format!("{}/ffi.rs", out_dir))
		.expect("Couldn't write bindings!");
}

fn compile_cmake() {
	let target = env::var("TARGET").unwrap();

	let mut config = cmake::Config::new("randomx");
	config.no_build_target(true);

	// Apple ARM targets
	if target.contains("apple") && target.contains("aarch64") {
		config.define("ARCH", "native");
		config.define("CMAKE_OSX_ARCHITECTURES", "arm64");

		// For iOS, set CMAKE_SYSTEM_NAME so CMakeLists.txt can skip
		// executables that cannot be built for the device target
		if target.contains("ios") {
			config.define("CMAKE_SYSTEM_NAME", "iOS");
		}
	}

	config.build();
}

fn exec_if_newer<F: Fn()>(inpath: &str, outpath: &str, build: F) {
	if let Ok(metadata) = fs::metadata(outpath) {
		let outtime = FileTime::from_last_modification_time(&metadata);

		let intime = FileTime::from_last_modification_time(
			&fs::metadata(inpath)
				.unwrap_or_else(|_| panic!("Path {} not found", inpath)),
		);

		let buildfiletime =
			FileTime::from_last_modification_time(&fs::metadata("build.rs").unwrap());

		if outtime > intime && outtime > buildfiletime {
			return;
		}
	}

	build();
}

fn main() {
	println!("Starting randomx build");

	let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
	let target = env::var("TARGET").expect("TARGET not set");
	let target_env =
		env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
	let profile =
		env::var("PROFILE").unwrap_or_else(|_| "release".to_string());

	fail_on_empty_directory("randomx");

	compile_cmake();

	exec_if_newer(
		"randomx",
		&format!("{}/build", out_dir),
		compile_cmake,
	);

	exec_if_newer(
		"randomx",
		&format!("{}/ffi.rs", out_dir),
		|| {
			generate_bindings(&out_dir);
		},
	);

	/*
	 * Do not use cfg!(target_env = "...") here.
	 *
	 * build.rs is compiled for and executed on the HOST During
	 * cross-compilation, Cargo's TARGET and CARGO_CFG_TARGET_ENV
	 * describe the destination platform we actually need to link for
	 */

	if target_env == "msvc" {
		/*
		 * Visual Studio-style CMake generators commonly place the
		 * resulting library in:
		 *
		 *     build/Debug
		 *     build/Release
		 */
		let cmake_profile = if profile == "debug" {
			"Debug"
		} else {
			"Release"
		};

		println!(
			"cargo:rustc-link-search=native={}/build/{}",
			out_dir,
			cmake_profile
		);

		println!("cargo:rustc-link-lib=static=randomx");
	} else {
		/*
		 * Unix-style generators and MinGW generally place librandomx
		 * directly in the CMake build directory
		 */
		println!(
			"cargo:rustc-link-search=native={}/build",
			out_dir
		);

		println!("cargo:rustc-link-lib=static=randomx");

		/*
		 * RandomX is implemented in C++, so explicitly link the
		 * appropriate C++ runtime for the destination platform
		 */

		if target.contains("apple") {
			// macOS / iOS use libc++
			println!("cargo:rustc-link-lib=dylib=c++");
		} else if target.contains("android") {
			// Android NDK uses LLVM libc++
			println!("cargo:rustc-link-lib=dylib=c++_shared");
		} else if target.contains("windows-gnu") {
			// MinGW-w64 uses libstdc++
			println!("cargo:rustc-link-lib=static=stdc++");
		} else if target.contains("linux") {
			// GNU/Linux uses libstdc++
			println!("cargo:rustc-link-lib=dylib=stdc++");
		} else if target.contains("freebsd")
			|| target.contains("openbsd")
			|| target.contains("netbsd")
			|| target.contains("dragonfly")
		{
			// BSD toolchains generally use libc++
			println!("cargo:rustc-link-lib=dylib=c++");
		} else {
			panic!(
				"unsupported RandomX target: TARGET={}, TARGET_ENV={}",
				target,
				target_env
			);
		}
	}
}

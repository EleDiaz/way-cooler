extern crate cc;
extern crate pkg_config;

use std::{env, fs, io::Write, path::Path, process::Command};

use wayland_scanner::{generate_code, Side};

const PROTOCOL_DIR: &str = "../protocols";

fn main() {
    dump_git_version();
    build_wayland_glib_interface();
    generate_custom_protocols();
}

/// Writes the current git hash to a file that is read by Way Cooler
///
/// If this is a release build, the file will be empty.
fn dump_git_version() {
    let out_dir = env::var("OUT_DIR").expect("Could not find out directory!");
    let dest_path = Path::new(&out_dir).join("git-version.txt");
    let mut f = fs::File::create(&dest_path)
        .expect("Could not write git version to out directory");
    if let Some(git_version) = git_version() {
        f.write_all(git_version.as_ref())
            .expect("Could not write to git version file");
    }
}

/// Gets the current hash of HEAD. Returns None if for some reason
/// that could not be retrieved (e.g not in a git repository)
fn git_version() -> Option<String> {
    if !in_release_commit() {
        Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .ok()
            .map(|output| output.stdout)
            .map(|hash| String::from_utf8_lossy(&hash).trim().into())
    } else {
        None
    }
}

/// Determines if the current HEAD is tagged with a release
fn in_release_commit() -> bool {
    let result = Command::new("git")
        .arg("describe")
        .arg("--exact-match")
        .arg("--tags")
        .arg("HEAD")
        .output()
        .unwrap();
    result.status.success()
}

/// Build the wayland-glib interface as a static library
fn build_wayland_glib_interface() {
    let glib = pkg_config::probe_library("glib-2.0").unwrap();
    let dbus = pkg_config::probe_library("dbus-1").unwrap();
    let wayland = pkg_config::probe_library("wayland-client").unwrap();

    let mut builder = cc::Build::new();

    let include_paths = glib
        .include_paths
        .iter()
        .chain(dbus.include_paths.iter())
        .chain(wayland.include_paths.iter());

    for path in include_paths {
        builder.include(path);
    }

    builder
        .file("src/wayland_glib_interface.c")
        .compile("wayland_glib_interface");
    println!("cargo:rustc-flags=-l wayland-client");
}

fn generate_custom_protocols() {
    let out_dir_str = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_str);

    let protocols =
        fs::read_dir(PROTOCOL_DIR).expect("../protocols does not exist");
    for protocol in protocols {
        let protocol = protocol.expect("Could not read protocol");
        let path = protocol.path();

        let is_xml = path
            .extension()
            .map(|extension| extension == "xml")
            .unwrap_or(false);
        if protocol.file_type().unwrap().is_dir() || !is_xml {
            continue;
        }

        let out_path = out_dir.join(
            path.with_extension("rs")
                .file_name()
                .expect("Could not extract file name")
        );

        generate_code(path, out_path, Side::Client);
    }

    println!("cargo:rerun-if-changed={}", PROTOCOL_DIR);
}

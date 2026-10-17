//! End-to-end tests that exercise the pipeline through real files.

use std::path::{Path, PathBuf};

use halftone_core::pipeline::{self, Job};
use halftone_core::{ColorMode, HalftoneSettings, HalftoneStyle};

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("halftone-tests");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Write a P6 PPM gradient with a colored disc, using no image dependency.
fn write_test_ppm(path: &Path, width: u32, height: u32) {
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    for y in 0..height {
        for x in 0..width {
            let mut r = (255 * x / width.max(1)) as u8;
            let mut g = (255 * y / height.max(1)) as u8;
            let mut b = 200;
            let dx = x as i32 - width as i32 / 2;
            let dy = y as i32 - height as i32 / 2;
            if dx * dx + dy * dy < (width as i32 / 5).pow(2) {
                r = 255 - r;
                g = 255 - g;
                b = 40;
            }
            bytes.extend_from_slice(&[r, g, b]);
        }
    }
    std::fs::write(path, bytes).expect("write ppm");
}

fn write_test_obj(path: &Path) {
    let text = "\
v -1 -1 -1
v 1 -1 -1
v 1 1 -1
v -1 1 -1
v -1 -1 1
v 1 -1 1
v 1 1 1
v -1 1 1
f 1 2 3 4
f 5 8 7 6
f 1 5 6 2
f 2 6 7 3
f 3 7 8 4
f 5 1 4 8
";
    std::fs::write(path, text).expect("write obj");
}

#[test]
fn renders_an_image_file_to_svg() {
    let input = temp_dir().join("integration-color.ppm");
    write_test_ppm(&input, 96, 96);

    let settings = HalftoneSettings {
        width: Some(64),
        height: Some(64),
        cell_size: 4.0,
        ..HalftoneSettings::default()
    };
    let result = pipeline::render(&Job::image(&input, settings)).expect("render image");

    assert_eq!((result.width, result.height), (64, 64));
    assert!(result.shapes > 0);
    assert!(result.svg.starts_with("<?xml"));
    assert!(result.svg.contains("viewBox=\"0 0 64 64\""));
    assert!(result.svg.contains("<circle"));
}

#[test]
fn cmyk_image_produces_four_ink_layers() {
    let input = temp_dir().join("integration-cmyk.ppm");
    write_test_ppm(&input, 96, 96);

    let settings = HalftoneSettings {
        color_mode: ColorMode::Cmyk,
        cell_size: 5.0,
        ..HalftoneSettings::default()
    };
    let result = pipeline::render(&Job::image(&input, settings)).expect("render cmyk");

    for id in ["cyan", "magenta", "yellow", "black"] {
        assert!(
            result.svg.contains(&format!("id=\"{id}\"")),
            "missing layer {id}"
        );
    }
}

#[test]
fn renders_an_obj_mesh_with_rotation() {
    let input = temp_dir().join("integration-cube.obj");
    write_test_obj(&input);

    let mut job = Job::mesh(
        &input,
        HalftoneSettings {
            style: HalftoneStyle::Dots,
            width: Some(80),
            height: Some(80),
            cell_size: 5.0,
            ..HalftoneSettings::default()
        },
    );
    job.camera.rotation = [25.0, 35.0, 0.0];

    let result = pipeline::render(&job).expect("render mesh");
    assert_eq!((result.width, result.height), (80, 80));
    assert!(result.shapes > 0);
    // A shaded cube must produce a range of dot sizes, not a flat fill.
    let circles: Vec<&str> = result
        .svg
        .match_indices("<circle")
        .map(|(i, _)| &result.svg[i..(i + 60).min(result.svg.len())])
        .collect();
    let unique: std::collections::HashSet<&str> = circles.into_iter().collect();
    assert!(unique.len() > 5, "expected varied dot sizes");
}

#[test]
fn rejects_invalid_settings() {
    let input = temp_dir().join("integration-invalid.ppm");
    write_test_ppm(&input, 16, 16);
    let settings = HalftoneSettings {
        cell_size: 0.0,
        ..HalftoneSettings::default()
    };
    assert!(pipeline::render(&Job::image(&input, settings)).is_err());
}

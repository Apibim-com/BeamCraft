// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Actual readback test, with explicit adapter-absent skip and failure for device/render errors.
use beamcraft_v01_viewport_spike::{
    Error,
    camera::Camera,
    render::{Gpu, Renderer, Strategy, Uploaded, wait},
    scene,
};
#[test]
fn lit_plate_pixels_and_strategy_equivalence() {
    let gpu = match pollster::block_on(Gpu::headless()) {
        Ok(gpu) => gpu,
        Err(Error::NoAdapter) => {
            eprintln!("SKIP headless render: no native wgpu adapter");
            return;
        }
        Err(e) => panic!("adapter/device failed: {e}"),
    };
    let info = gpu.adapter.get_info();
    eprintln!(
        "Pixel test: {} / {:?} / {:?}",
        info.name, info.device_type, info.backend
    );
    let mut invalid = scene::build(1).expect("real kernel plate");
    invalid.objects.first_mut().expect("object").translation = [f32::NAN; 3];
    assert!(matches!(
        Uploaded::new(&gpu.device, &gpu.queue, &invalid, Strategy::Instanced),
        Err(Error::Input(_))
    ));
    let mut scene = scene::build(1).expect("real kernel plate");
    scene.target = [32.5, 35.0, 3.0];
    let camera = Camera::new(scene.target, 100.0)
        .expect("camera")
        .view(1.0)
        .expect("view");
    let renderer = Renderer::new(&gpu.adapter, &gpu.device, 128, 128).expect("renderer");
    let mut images = Vec::new();
    for strategy in [Strategy::Instanced, Strategy::Merged] {
        let data = Uploaded::new(&gpu.device, &gpu.queue, &scene, strategy).expect("upload");
        let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let i = renderer.draw(&gpu.device, &gpu.queue, &data, camera);
        wait(&gpu.device, i).expect("completion");
        let (pixels, row) = renderer
            .readback(&gpu.device, &gpu.queue)
            .expect("readback");
        assert!(pollster::block_on(scope.pop()).is_none());
        let pixel = |x: usize, y: usize| {
            pixels
                .get(y * row as usize + x * 4..y * row as usize + x * 4 + 4)
                .expect("pixel")
        };
        for (x, y) in [(0, 0), (127, 0), (0, 127), (127, 127)] {
            let [r, g, b, a] = *pixel(x, y) else {
                panic!("rgba")
            };
            assert!(
                (4..=6).contains(&r) && (7..=9).contains(&g) && (12..=14).contains(&b) && a == 255,
                "background {r} {g} {b} {a}"
            );
        }
        let [r, g, b, a] = *pixel(64, 64) else {
            panic!("rgba")
        };
        assert!(
            r > 20 && g > 60 && b > 100 && r < g && g < b && a == 255,
            "lit plate {r} {g} {b} {a}"
        );
        images.push(pixels);
    }
    let a = images.first().expect("instanced");
    let b = images.get(1).expect("merged");
    assert!(
        a.iter().zip(b).all(|(a, b)| a.abs_diff(*b) <= 1),
        "equivalent scene pixels"
    );
}

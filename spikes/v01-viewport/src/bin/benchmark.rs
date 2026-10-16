// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Native offscreen benchmark. JSON goes to stdout and optionally --output PATH.
use beamcraft_v01_viewport_spike::{
    Error,
    camera::Camera,
    json_string,
    render::{Gpu, Renderer, Strategy, Uploaded, wait},
    scene,
};
use std::time::Instant;
fn stats(mut values: Vec<f64>) -> Result<String, Error> {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n == 0 {
        return Err(Error::Input("empty samples"));
    }
    let median = values.get(n / 2).ok_or(Error::Input("median"))?;
    let p95 = values
        .get((n * 95).div_ceil(100) - 1)
        .ok_or(Error::Input("p95"))?;
    let max = values.last().ok_or(Error::Input("maximum"))?;
    Ok(format!(
        "{{\"median\":{median:.6},\"p95\":{p95:.6},\"max\":{max:.6}}}"
    ))
}
fn run() -> Result<(), Error> {
    let mut output = None;
    let mut only_count = None;
    let mut only_strategy = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(args.next().ok_or(Error::Input("missing output path"))?),
            "--count" => {
                only_count = Some(
                    args.next()
                        .ok_or(Error::Input("missing count"))?
                        .parse::<u32>()
                        .map_err(|_| Error::Input("count"))?,
                )
            }
            "--strategy" => {
                only_strategy = Some(match args.next().as_deref() {
                    Some("instanced") => Strategy::Instanced,
                    Some("merged") => Strategy::Merged,
                    _ => return Err(Error::Input("strategy: instanced or merged")),
                })
            }
            _ => {
                return Err(Error::Input(
                    "options: --output PATH --count 1..100000 --strategy instanced|merged",
                ));
            }
        }
    }
    let gpu = pollster::block_on(Gpu::headless())?;
    let info = gpu.adapter.get_info();
    eprintln!(
        "Adapter: {} / {:?} / {:?}; software={}",
        info.name,
        info.device_type,
        info.backend,
        info.device_type == wgpu::DeviceType::Cpu
    );
    let renderer = Renderer::new(&gpu.adapter, &gpu.device, 1024, 768)?;
    let mut rows = Vec::new();
    for count in only_count.map_or_else(|| vec![1_000, 10_000, 100_000], |c| vec![c]) {
        let start = Instant::now();
        let scene = scene::build(count)?;
        let build_ms = start.elapsed().as_secs_f64() * 1000.0;
        let camera = Camera::new(scene.target, scene.extent)?.view(1024.0 / 768.0)?;
        for strategy in [Strategy::Instanced, Strategy::Merged] {
            if only_strategy.is_some_and(|s| s != strategy) {
                continue;
            }
            let data = Uploaded::new(&gpu.device, &gpu.queue, &scene, strategy)?;
            let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
            for _ in 0..30 {
                let i = renderer.draw(&gpu.device, &gpu.queue, &data, camera);
                wait(&gpu.device, i)?;
            }
            let mut submit = Vec::with_capacity(300);
            let mut complete = Vec::with_capacity(300);
            for _ in 0..300 {
                let start = Instant::now();
                let i = renderer.draw(&gpu.device, &gpu.queue, &data, camera);
                submit.push(start.elapsed().as_secs_f64() * 1000.0);
                wait(&gpu.device, i)?;
                complete.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            if let Some(e) = pollster::block_on(scope.pop()) {
                return Err(Error::Graphics(e.to_string()));
            }
            rows.push(format!("{{\"count\":{count},\"strategy\":{},\"scene_build_ms\":{build_ms:.6},\"pack_ms\":{:.6},\"upload_bytes\":{},\"upload_ms\":{:.6},\"frames\":300,\"warmup_frames\":30,\"draw_submit_ms\":{},\"frame_complete_ms\":{},\"peak_process_memory_bytes\":null}}",json_string(strategy.name()),data.pack_ms,data.bytes,data.upload_ms,stats(submit)?,stats(complete)?));
            eprintln!("Completed {count} {}", strategy.name());
        }
    }
    let profile = if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    };
    let json = format!(
        "{{\"schema\":1,\"adapter\":{{\"name\":{},\"device_type\":{},\"backend\":{},\"driver\":{},\"driver_info\":{},\"software\":{}}},\"profile\":{},\"os\":{},\"arch\":{},\"target\":[1024,768],\"msaa_samples\":{},\"timing\":\"CPU wall clock, serialized submit and device completion; no timestamp queries\",\"peak_memory\":\"per-case unavailable; optional Windows wrapper records whole-run peak working set\",\"results\":[{}]}}\n",
        json_string(&info.name),
        json_string(&format!("{:?}", info.device_type)),
        json_string(&format!("{:?}", info.backend)),
        json_string(&info.driver),
        json_string(&info.driver_info),
        info.device_type == wgpu::DeviceType::Cpu,
        json_string(profile),
        json_string(std::env::consts::OS),
        json_string(std::env::consts::ARCH),
        renderer.samples,
        rows.join(",")
    );
    if let Some(path) = output {
        std::fs::write(path, &json).map_err(Error::Io)?;
    }
    print!("{json}");
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}

//! Headless N01 experiment entry point, contributor-original MPL-2.0.
#![forbid(unsafe_code)]
use beamcraft_n01_openusd_spike::{Result, author_model, failure};
use openusd::{sdf, usd, usda, usdc};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

fn parse_case(path: &Path) -> Result<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(65_537).read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err(failure("input exceeds 64 KiB experiment cap"));
    }
    let layer = sdf::Layer::from_bytes("n01-hostile", bytes)?;
    let paths = layer.data().spec_paths();
    if paths.len() > 10_000 {
        return Err(failure("post-parse spec cap"));
    }
    let mut fields = 0_usize;
    for path in paths {
        for field in layer
            .data()
            .list_fields(&path)
            .ok_or_else(|| failure("missing field list"))?
        {
            fields = fields.saturating_add(1);
            if fields > 100_000 {
                return Err(failure("post-parse field cap"));
            }
            let _value = layer.data().get_field(&path, &field)?;
        }
    }
    Ok(format!("ACCEPTED fields={fields}"))
}

/// Writes the deterministic synthetic model as USDC, the seed for the binary mutation cases.
/// The input is our own authored model, not untrusted bytes, so no containment gate is needed.
fn seed(directory: &Path) -> Result<()> {
    fs::create_dir_all(directory)?;
    let text_path = directory.join("model.usda");
    author_model(&text_path)?;
    let data = usda::parse(&fs::read_to_string(&text_path)?)?;
    usdc::CrateWriter::write_to_file(&data, directory.join("model.usdc"))?;
    Ok(())
}

fn sanity(count: usize, directory: &Path) -> Result<()> {
    if !matches!(count, 1000 | 10_000) {
        return Err(failure("sanity count must be 1000 or 10000"));
    }
    fs::create_dir_all(directory)?;
    let mut text = String::from(
        "#usda 1.0\n( metersPerUnit = 0.001\n upAxis = \"Z\" )\ndef Xform \"World\" { def Xform \"Steel\" {\n",
    );
    for number in 0..count {
        let id = format!("0199e7a0-0000-7000-8000-{number:012x}");
        text.push_str(&format!("def Xform \"E_{}\" {{ custom string bim:id = \"{id}\"\n custom double steel:plate:widthMm = 1000.125\n }}\n", id.replace('-', "")));
    }
    text.push_str("} }\n");
    let model = directory.join("model.usda");
    fs::write(&model, text)?;
    let root = directory.join("project.usda");
    fs::write(&root, "#usda 1.0\n( subLayers = [@model.usda@] )\n")?;
    let start = Instant::now();
    let stage = usd::Stage::open(&root.to_string_lossy())?;
    let open = start.elapsed();
    let traverse_start = Instant::now();
    let mut paths = Vec::new();
    stage.traverse(usd::PrimPredicate::DEFAULT, |path| {
        // Count is fixed by this generator; cap collection independently anyway.
        if paths.len() < 10_003 {
            paths.push(path.to_string());
        }
    })?;
    let mut elements = 0_usize;
    let mut sum = 0_f64;
    for path in paths {
        if path.starts_with("/World/Steel/E_") {
            elements = elements.saturating_add(1);
            sum += stage
                .attribute(format!("{path}.steel:plate:widthMm"))?
                .get::<f64>()?
                .ok_or_else(|| failure("missing width"))?;
        }
    }
    let traverse = traverse_start.elapsed();
    if elements != count || sum != (count as f64) * 1000.125 {
        return Err(failure("sanity traversal/value mismatch"));
    }
    let save_start = Instant::now();
    let layer_id = stage
        .layer_identifiers()
        .into_iter()
        .find(|id| id.ends_with("model.usda"))
        .ok_or_else(|| failure("model layer missing"))?;
    let layer = stage
        .layer(&layer_id)
        .ok_or_else(|| failure("layer view missing"))?;
    layer.export(directory.join("saved.usda").to_string_lossy())?;
    let save = save_start.elapsed();
    println!(
        "SANITY elements={elements} open_ms={:.3} compose_traverse_ms={:.3} save_ms={:.3} sum={sum}",
        open.as_secs_f64() * 1000.0,
        traverse.as_secs_f64() * 1000.0,
        save.as_secs_f64() * 1000.0
    );
    Ok(())
}

fn run() -> Result<ExitCode> {
    let args: Vec<_> = std::env::args().take(6).collect();
    let command = args
        .get(1)
        .ok_or_else(|| failure("use parse PATH, seed DIRECTORY or sanity COUNT DIRECTORY"))?;
    if command == "seed" {
        let directory = args.get(2).ok_or_else(|| failure("missing directory"))?;
        seed(Path::new(directory))?;
    } else if command == "parse" {
        // Parent assigns the memory-limited Job before sending this byte.
        let mut gate = [0_u8; 1];
        std::io::stdin().read_exact(&mut gate)?;
        if gate != [b'G'] {
            return Err(failure("supervisor gate"));
        }
        let path = args.get(2).ok_or_else(|| failure("missing input path"))?;
        let result = std::panic::catch_unwind(|| parse_case(Path::new(path)));
        match result {
            Ok(Ok(message)) => println!("{message}"),
            Ok(Err(error)) => println!(
                "REJECTED {}",
                error.to_string().chars().take(350).collect::<String>()
            ),
            Err(_) => {
                println!("PANIC");
                return Ok(ExitCode::from(2));
            }
        }
    } else if command == "sanity" {
        let mut gate = [0_u8; 1];
        std::io::stdin().read_exact(&mut gate)?;
        if gate != [b'G'] {
            return Err(failure("supervisor gate"));
        }
        let count = args
            .get(2)
            .ok_or_else(|| failure("missing count"))?
            .parse()?;
        let directory = args.get(3).ok_or_else(|| failure("missing directory"))?;
        sanity(count, Path::new(directory))?;
    } else {
        return Err(failure("unknown command"));
    }
    std::io::stdout().flush()?;
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

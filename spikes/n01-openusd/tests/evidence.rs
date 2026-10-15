//! Original synthetic observations; no upstream tests or fixtures copied. MPL-2.0.
use beamcraft_n01_openusd_spike::*;
use openusd::{sdf, usd, usda, usdc};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

fn directory(name: &str) -> Result<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("artifacts")
        .join(format!("test-{}-{name}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn snapshot(data: &dyn sdf::AbstractData) -> Result<BTreeMap<(String, String), sdf::Value>> {
    let mut result = BTreeMap::new();
    for path in data.spec_paths() {
        for field in data
            .list_fields(&path)
            .ok_or_else(|| failure("fields absent"))?
        {
            result.insert(
                (path.to_string(), field.clone()),
                data.get_field(&path, &field)?.into_owned(),
            );
        }
    }
    Ok(result)
}

#[test]
fn usda_usdc_roundtrip_every_authored_field_and_second_write() -> Result<()> {
    let dir = directory("roundtrip")?;
    let original = dir.join("model.usda");
    author_model(&original)?;
    let text = fs::read_to_string(&original)?;
    structural_check(&text)?;
    let data = usda::parse(&text)?;
    let expected = snapshot(&data)?;
    let second_text = usda::TextWriter::write_to_string(&data)?;
    assert_eq!(text, second_text, "USDA second write");
    let binary = dir.join("model.usdc");
    usdc::CrateWriter::write_to_file(&data, &binary)?;
    let bytes = fs::read(&binary)?;
    let decoded = usdc::CrateData::open(bytes.clone(), true)?;
    assert_eq!(snapshot(&decoded)?, expected, "USDC authored values");
    let second_binary = dir.join("second.usdc");
    usdc::CrateWriter::write_to_file(&decoded, &second_binary)?;
    assert_eq!(bytes, fs::read(second_binary)?, "USDC second write");
    for file in [&original, &binary] {
        let stage = usd::Stage::open(&file.to_string_lossy())?;
        assert_eq!(
            stage.stage_metadata("metersPerUnit")?,
            Some(sdf::Value::Double(0.001))
        );
        assert_eq!(
            stage.stage_metadata("upAxis")?,
            Some(sdf::Value::token("Z"))
        );
        assert_eq!(
            stage
                .attribute(format!("{ELEMENT_PATH}.bim:id"))?
                .get::<String>()?
                .as_deref(),
            Some(ELEMENT_ID)
        );
        assert_eq!(
            stage
                .attribute(format!("{ELEMENT_PATH}.steel:plate:widthMm"))?
                .get::<f64>()?
                .map(f64::to_bits),
            Some(1000.125_f64.to_bits())
        );
        assert_eq!(
            stage
                .relationship(format!("{ELEMENT_PATH}.bim:material"))?
                .targets()?,
            vec![sdf::path("/World/Materials/Synthetic")?]
        );
        assert_eq!(
            stage
                .attribute(format!("{ELEMENT_PATH}.future:unknown"))?
                .get::<String>()?
                .as_deref(),
            Some("keep this exact string")
        );
        assert_eq!(
            stage
                .prim(ELEMENT_PATH)?
                .get_metadata::<sdf::Value>("customData")?,
            Some(sdf::Value::Dictionary(
                [(
                    "future:note".to_owned(),
                    sdf::Value::String("untouched".to_owned())
                )]
                .into()
            ))
        );
        assert_eq!(
            stage.custom_layer_data()?,
            Some(sdf::Value::Dictionary(
                [("future:layer".to_owned(), sdf::Value::Int(73))].into()
            ))
        );
    }
    println!(
        "ROUNDTRIP USDA={} bytes USDC={} bytes fields={} exact values/metadata/targets/unknowns and second-write equality",
        text.len(),
        bytes.len(),
        expected.len()
    );
    Ok(())
}

#[test]
fn owned_trait_edit_preserves_unknown_content_and_identity() -> Result<()> {
    let dir = directory("adapter")?;
    let source = dir.join("model.usda");
    author_model(&source)?;
    let before = usda::read_file(&source)?;
    let backend = RustBackend::open(&source, std::slice::from_ref(&source))?;
    let api: &dyn LayerBackend = &backend;
    assert_eq!(api.read_number(ELEMENT_ID, NumberField::WidthMm)?, 1000.125);
    assert!(
        api.write_number(ELEMENT_ID, NumberField::WidthMm, f64::NAN)
            .is_err()
    );
    assert!(
        api.write_number("short", NumberField::WidthMm, 42.0)
            .is_err()
    );
    api.write_number(ELEMENT_ID, NumberField::WidthMm, 1500.25)?;
    let output = dir.join("edited.usda");
    api.export_edit_layer(&output)?;
    assert!(api.export_edit_layer(&source).is_err());
    let after = usda::read_file(&output)?;
    let mut expected = snapshot(&before)?;
    expected.insert(
        (
            format!("{ELEMENT_PATH}.steel:plate:widthMm"),
            "default".to_owned(),
        ),
        sdf::Value::Double(1500.25),
    );
    assert_eq!(snapshot(&after)?, expected);
    let fresh = RustBackend::open(&output, std::slice::from_ref(&output))?;
    assert_eq!(
        fresh.read_number(ELEMENT_ID, NumberField::WidthMm)?,
        1500.25
    );
    println!(
        "ADAPTER one width edit; all other authored fields retained; bad ID/nonfinite/source overwrite rejected"
    );
    Ok(())
}

fn opinions(width: f64) -> String {
    format!(
        "#usda 1.0\nover \"World\" {{ over \"Steel\" {{ over \"E_0199e7a0000070008000000000000001\" {{ custom double steel:plate:widthMm = {width}\n }} }} }}\n"
    )
}

#[test]
fn sublayers_strong_over_and_active_false() -> Result<()> {
    let dir = directory("composition")?;
    author_model(&dir.join("model.usda"))?;
    fs::write(dir.join("strong.usda"), opinions(1200.5))?;
    fs::write(dir.join("middle.usda"), opinions(1100.5))?;
    let root = dir.join("project.usda");
    fs::write(
        &root,
        "#usda 1.0\n( subLayers = [@strong.usda@, @middle.usda@, @model.usda@] )\n",
    )?;
    let stage = usd::Stage::open(&root.to_string_lossy())?;
    let width = || {
        stage
            .attribute(format!("{ELEMENT_PATH}.steel:plate:widthMm"))?
            .get::<f64>()
    };
    assert_eq!(width()?, Some(1200.5));
    let strong_id = stage
        .layer_identifiers()
        .into_iter()
        .find(|p| p.ends_with("strong.usda"))
        .ok_or_else(|| failure("strong layer not found"))?;
    stage.mute_layer(strong_id);
    assert_eq!(width()?, Some(1100.5));
    stage.override_prim(ELEMENT_PATH)?;
    stage
        .attribute(format!("{ELEMENT_PATH}.steel:plate:widthMm"))?
        .set(1300.5_f64)?;
    assert_eq!(width()?, Some(1300.5));
    let mut traversed = BTreeSet::new();
    stage.traverse(usd::PrimPredicate::DEFAULT, |p| {
        traversed.insert(p.to_string());
    })?;
    assert!(!traversed.contains(OTHER_PATH));
    stage.prim(OTHER_PATH)?.set_active(true)?;
    traversed.clear();
    stage.traverse(usd::PrimPredicate::DEFAULT, |p| {
        traversed.insert(p.to_string());
    })?;
    assert!(traversed.contains(OTHER_PATH));
    assert!(stage.composition_errors().is_empty());
    stage
        .root_layer()
        .export(dir.join("edited-root.usda").to_string_lossy())?;
    println!(
        "COMPOSITION widths 1200.5 -> 1100.5 -> 1300.5; inactive excluded/reactivated included; zero composition diagnostics"
    );
    Ok(())
}

#[test]
fn foreign_reference_bytes_unchanged_but_raw_override_allowed_wrapper_rejects() -> Result<()> {
    let dir = directory("foreign")?;
    let foreign = dir.join("foreign.usda");
    fs::write(
        &foreign,
        format!(
            "#usda 1.0\ndef Xform \"Source\" {{ custom string bim:id = \"{OTHER_ID}\"\n custom double steel:plate:widthMm = 77\n }}\n"
        ),
    )?;
    let bytes = fs::read(&foreign)?;
    let original_permissions = fs::metadata(&foreign)?.permissions();
    let mut read_only = original_permissions.clone();
    read_only.set_readonly(true);
    fs::set_permissions(&foreign, read_only)?;
    let root = dir.join("project.usda");
    fs::write(
        &root,
        "#usda 1.0\ndef Xform \"World\" { def Xform \"Steel\" { def Xform \"E_0199e7a0000070008000000000000002\" ( prepend references = @foreign.usda@</Source> ) {} } }\n",
    )?;
    let backend = RustBackend::open(&root, std::slice::from_ref(&root))?;
    let rejected = backend
        .write_number(OTHER_ID, NumberField::WidthMm, 99.0)
        .err()
        .ok_or_else(|| failure("foreign edit accepted"))?;
    assert!(rejected.to_string().contains("FOREIGN_READ_ONLY"));
    let stage = usd::Stage::open(&root.to_string_lossy())?;
    assert_eq!(
        stage
            .attribute(format!("{OTHER_PATH}.steel:plate:widthMm"))?
            .get::<f64>()?,
        Some(77.0)
    );
    let sites = stage.prim(OTHER_PATH)?.prim_stack()?;
    assert!(
        sites
            .iter()
            .any(|site| site.layer.ends_with("foreign.usda"))
    );
    stage
        .attribute(format!("{OTHER_PATH}.steel:plate:widthMm"))?
        .set(99_f64)?;
    assert_eq!(
        stage
            .attribute(format!("{OTHER_PATH}.steel:plate:widthMm"))?
            .get::<f64>()?,
        Some(99.0)
    );
    stage
        .root_layer()
        .export(dir.join("override.usda").to_string_lossy())?;
    assert_eq!(fs::read(&foreign)?, bytes);
    assert!(fs::metadata(&foreign)?.permissions().readonly());
    fs::set_permissions(&foreign, original_permissions)?;
    println!(
        "FOREIGN width 77 -> raw local override 99 accepted; foreign bytes identical; wrapper rejects: {rejected}"
    );
    Ok(())
}

fn keys(diffs: &[usd::Diff]) -> Result<BTreeSet<FieldKey>> {
    let mut fields = Vec::new();
    for diff in diffs {
        if diff.mapping.is_some() {
            return Err(failure("mapping outside this experiment"));
        }
        for edit in &diff.edits {
            match edit {
                usd::Edit::SetField { path, field, .. } => {
                    fields.push((path.to_string(), field.to_string()))
                }
                _ => return Err(failure("non-field diff outside experiment")),
            }
        }
    }
    normalized_keys_for_edits(
        &fields,
        &[(ELEMENT_PATH.to_owned(), ELEMENT_ID.to_owned())].into(),
    )
}

#[test]
fn captured_deltas_replay_and_detect_same_identity_attribute_conflict() -> Result<()> {
    let dir = directory("delta")?;
    let base = dir.join("base.usda");
    author_model(&base)?;
    let left = usd::ReplayStage::from(usd::Stage::open(&base.to_string_lossy())?);
    left.attribute(format!("{ELEMENT_PATH}.steel:plate:widthMm"))?
        .set(1200.5_f64)?;
    let width_diffs = left.diff();
    left.clear();
    left.attribute(format!("{ELEMENT_PATH}.bim:name"))?
        .set("Renamed".to_owned())?;
    let label_diffs = left.diff();
    let right = usd::ReplayStage::from(usd::Stage::open(&base.to_string_lossy())?);
    right
        .attribute(format!("{ELEMENT_PATH}.steel:plate:widthMm"))?
        .set(1400.5_f64)?;
    let competing_diffs = right.diff();
    let a = keys(&width_diffs)?;
    let b = keys(&competing_diffs)?;
    let c = keys(&label_diffs)?;
    assert_eq!(a.intersection(&b).count(), 1);
    assert_eq!(a.intersection(&c).count(), 0);
    let mirror = usd::Stage::open(&base.to_string_lossy())?;
    for diff in width_diffs.iter().chain(label_diffs.iter()) {
        mirror.apply_diff(diff, usd::ApplyMode::CurrentEditTarget)?;
    }
    assert_eq!(
        snapshot(mirror.root_layer().data())?,
        snapshot(left.root_layer().data())?
    );
    assert_eq!(
        mirror
            .attribute(format!("{ELEMENT_PATH}.bim:id"))?
            .get::<String>()?
            .as_deref(),
        Some(ELEMENT_ID)
    );
    fs::write(
        dir.join("captured-diffs.txt"),
        format!(
            "width={width_diffs:#?}\nlabel={label_diffs:#?}\ncompeting={competing_diffs:#?}\nkeys={a:#?}\n"
        ),
    )?;
    println!(
        "DELTA width diffs={} label diffs={}; replay exact authored snapshot; same-field collision=1 different-field collision=0",
        width_diffs.len(),
        label_diffs.len()
    );
    Ok(())
}

#[test]
fn identity_and_structural_guards() -> Result<()> {
    assert_eq!(element_path(ELEMENT_ID)?, ELEMENT_PATH);
    for invalid in [
        "",
        "0199e7a0",
        "0199e7a0-0000-4000-8000-000000000001",
        "0199e7a0-0000-7000-0000-000000000001",
    ] {
        assert!(element_path(invalid).is_err());
    }
    structural_check("#usda 1.0\ndef \"A\" { string s = \"} # [\" }\n")?;
    assert!(structural_check("#usda 1.0\ndef \"A\" {").is_err());
    assert!(structural_check("#usda 1.0\n{]").is_err());
    Ok(())
}

#[test]
fn unknown_prim_metadata_survives_text_and_binary() -> Result<()> {
    use sdf::AbstractData;
    let text = "#usda 1.0\ndef Xform \"A\" ( futureThing = \"opaque\" ) { custom string future:attribute = \"kept\"\n}\n";
    let data = usda::parse(text)?;
    assert!(data.has_field(&sdf::path("/A")?, "futureThing"));
    assert_eq!(
        data.get_field(&sdf::path("/A.future:attribute")?, "default")?
            .as_ref(),
        &sdf::Value::String("kept".to_owned())
    );
    let rewritten = usda::TextWriter::write_to_string(&data)?;
    assert!(rewritten.contains("futureThing") && rewritten.contains("opaque"));
    let parsed = usda::parse(&rewritten)?;
    assert_eq!(snapshot(&data)?, snapshot(&parsed)?);
    let dir = directory("unknown-metadata")?;
    let binary = dir.join("unknown.usdc");
    usdc::CrateWriter::write_to_file(&data, &binary)?;
    let decoded = usdc::CrateData::open(fs::read(binary)?, true)?;
    assert_eq!(snapshot(&data)?, snapshot(&decoded)?);
    println!("UNKNOWN unregistered prim metadata and namespaced string survive USDA and USDC");
    Ok(())
}

#[test]
fn hostile_nonfinite_value_is_not_engine_validity() -> Result<()> {
    use sdf::AbstractData;
    let data = usda::parse("#usda 1.0\ndef \"A\" { double x = 1e999999999\n}\n")?;
    let value = data.get_field(&sdf::path("/A.x")?, "default")?;
    assert_eq!(value.as_ref(), &sdf::Value::Double(f64::INFINITY));
    println!(
        "HOSTILE numeric overflow accepted as positive infinity; BeamCraft must reject nonfinite engineering input"
    );
    Ok(())
}

#[test]
fn adapter_contract_accepts_a_std_only_stub() -> Result<()> {
    struct Stub(std::cell::Cell<f64>);
    impl LayerBackend for Stub {
        fn read_number(&self, id: &str, _field: NumberField) -> Result<f64> {
            element_path(id)?;
            Ok(self.0.get())
        }
        fn write_number(&self, id: &str, _field: NumberField, value: f64) -> Result<()> {
            element_path(id)?;
            self.0.set(value);
            Ok(())
        }
        fn export_edit_layer(&self, _destination: &std::path::Path) -> Result<()> {
            Err(failure("stub has no serialization"))
        }
    }
    fn contract(api: &dyn LayerBackend) -> Result<()> {
        api.write_number(ELEMENT_ID, NumberField::WidthMm, 1234.5)?;
        assert_eq!(api.read_number(ELEMENT_ID, NumberField::WidthMm)?, 1234.5);
        Ok(())
    }
    let dir = directory("containment")?;
    let model = dir.join("model.usda");
    author_model(&model)?;
    contract(&RustBackend::open(&model, std::slice::from_ref(&model))?)?;
    contract(&Stub(std::cell::Cell::new(0.0)))?;
    println!(
        "CONTAINMENT same owned numeric trait exercise on Rust backend and std-only stub; no upgrade or ownership assignment exercise"
    );
    Ok(())
}

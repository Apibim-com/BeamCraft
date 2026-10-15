//! Contributor-original, MPL-2.0 N01 experiments. Not a production USD adapter.
#![forbid(unsafe_code)]

use openusd::{sdf, usd, usda};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};

/// Fallible experiment result; no upstream error type in the adapter surface.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
/// Synthetic, complete UUIDv7; no client/project identity is used.
pub const ELEMENT_ID: &str = "0199e7a0-0000-7000-8000-000000000001";
/// Identity-preserving F04 path for the synthetic element.
pub const ELEMENT_PATH: &str = "/World/Steel/E_0199e7a0000070008000000000000001";
/// Independent second synthetic identity.
pub const OTHER_ID: &str = "0199e7a0-0000-7000-8000-000000000002";
/// Second element's full path.
pub const OTHER_PATH: &str = "/World/Steel/E_0199e7a0000070008000000000000002";

/// Create a diagnostic without panicking.
pub fn failure(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}

/// Bounded identity validation for this experiment, following F04/RFC 9562 layout.
pub fn element_path(id: &str) -> Result<String> {
    let bytes = id.as_bytes();
    if bytes.len() != 36 {
        return Err(failure("INVALID_ID: expected a complete UUIDv7"));
    }
    for (position, byte) in bytes.iter().enumerate() {
        if matches!(position, 8 | 13 | 18 | 23) {
            if *byte != b'-' {
                return Err(failure("INVALID_ID: separator"));
            }
        } else if !byte.is_ascii_digit() && !(b'a'..=b'f').contains(byte) {
            return Err(failure("INVALID_ID: lowercase hex required"));
        }
    }
    if bytes.get(14) != Some(&b'7') || !matches!(bytes.get(19), Some(b'8' | b'9' | b'a' | b'b')) {
        return Err(failure("INVALID_ID: UUID version/variant"));
    }
    Ok(format!("/World/Steel/E_{}", id.replace('-', "")))
}

/// The sole numeric field accepted by the spike mutation surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NumberField {
    /// Explicit millimetres; never a schema fallback.
    WidthMm,
}
impl NumberField {
    fn name(self) -> &'static str {
        match self {
            Self::WidthMm => "steel:plate:widthMm",
        }
    }
}

/// Minimal owned trait. No stage, path setter, diff or upstream handle escapes.
pub trait LayerBackend {
    /// Read an explicitly authored owned value.
    fn read_number(&self, id: &str, field: NumberField) -> Result<f64>;
    /// Author an allowed field after provenance and range checks.
    fn write_number(&self, id: &str, field: NumberField, value: f64) -> Result<()>;
    /// Export only the owned root edit layer to a host-selected destination.
    fn export_edit_layer(&self, destination: &Path) -> Result<()>;
}

/// Native spike backend; deliberately stays out of the repository workspace.
pub struct RustBackend {
    stage: usd::Stage,
    owned_layers: BTreeSet<PathBuf>,
}
impl RustBackend {
    /// Open trusted synthetic fixtures with host-selected owned layer identities.
    /// This is not an untrusted-path admission or a contained production resolver.
    pub fn open(root: &Path, owned_layers: &[PathBuf]) -> Result<Self> {
        if owned_layers.len() > 128 {
            return Err(failure("LIMIT_EXCEEDED: owned layer list"));
        }
        let owned_layers = owned_layers
            .iter()
            .map(std::fs::canonicalize)
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        Ok(Self {
            stage: usd::Stage::open(&root.to_string_lossy())?,
            owned_layers,
        })
    }

    fn owned_path(&self, id: &str) -> Result<String> {
        let path = element_path(id)?;
        let sites = self.stage.prim(path.as_str())?.prim_stack()?;
        if sites.is_empty() || sites.len() > 128 {
            return Err(failure("FOREIGN_READ_ONLY: unknown ownership"));
        }
        for site in sites {
            if !self
                .owned_layers
                .contains(&std::fs::canonicalize(&site.layer)?)
            {
                return Err(failure("FOREIGN_READ_ONLY: contributing foreign layer"));
            }
        }
        let authored_id = self
            .stage
            .attribute(format!("{path}.bim:id"))?
            .get::<String>()?;
        if authored_id.as_deref() != Some(id) {
            return Err(failure("INVALID_ID: path/id mismatch"));
        }
        let root = std::fs::canonicalize(self.stage.root_layer().identifier())?;
        if !self.owned_layers.contains(&root) {
            return Err(failure("FOREIGN_READ_ONLY: edit layer"));
        }
        Ok(path)
    }
}
impl LayerBackend for RustBackend {
    fn read_number(&self, id: &str, field: NumberField) -> Result<f64> {
        let path = self.owned_path(id)?;
        let attribute = self.stage.attribute(format!("{path}.{}", field.name()))?;
        if !attribute.has_authored_value()? {
            return Err(failure("INVALID_ARGUMENT: missing explicit input"));
        }
        attribute
            .get::<f64>()?
            .ok_or_else(|| failure("missing value"))
    }
    fn write_number(&self, id: &str, field: NumberField, value: f64) -> Result<()> {
        if !value.is_finite() || value <= 0.0 || value > 1_000_000.0 {
            return Err(failure(
                "INVALID_ARGUMENT: finite bounded dimension required",
            ));
        }
        let path = self.owned_path(id)?;
        self.stage
            .attribute(format!("{path}.{}", field.name()))?
            .set(value)?;
        Ok(())
    }
    fn export_edit_layer(&self, destination: &Path) -> Result<()> {
        let root = std::fs::canonicalize(self.stage.root_layer().identifier())?;
        if !self.owned_layers.contains(&root) {
            return Err(failure("FOREIGN_READ_ONLY: export source"));
        }
        // Host supplies a fresh output path: never overwrite a composed source.
        if destination.exists() {
            return Err(failure("refuse to overwrite an existing layer"));
        }
        self.stage
            .root_layer()
            .export(destination.to_string_lossy())?;
        Ok(())
    }
}

/// Construct a fully original static fixture with the crate's authoring API.
pub fn author_model(destination: &Path) -> Result<()> {
    let stage = usd::Stage::builder().in_memory("n01-original-model.usda")?;
    for path in [
        "/World",
        "/World/Steel",
        "/World/Materials",
        "/World/Materials/Synthetic",
    ] {
        stage.define_prim(path)?.set_type_name("Xform")?;
    }
    stage.set_default_prim("World")?;
    for (path, id) in [(ELEMENT_PATH, ELEMENT_ID), (OTHER_PATH, OTHER_ID)] {
        stage.define_prim(path)?.set_type_name("Xform")?;
        for (name, text) in [
            ("bim:id", id),
            ("bim:name", "Original plate"),
            ("bim:elementKind", "plate"),
            ("bim:geometryMode", "parametric"),
            ("bim:schemaVersion", "0.1.0"),
            ("future:unknown", "keep this exact string"),
        ] {
            stage
                .attribute_builder(format!("{path}.{name}"), "string")
                .custom(true)
                .set(text.to_owned())
                .build()?;
        }
        for (name, value) in [
            ("widthMm", 1000.125_f64),
            ("lengthMm", 500.25),
            ("thicknessMm", 10.5),
        ] {
            stage
                .attribute_builder(format!("{path}.steel:plate:{name}"), "double")
                .custom(true)
                .set(value)
                .build()?;
        }
        stage
            .create_relationship(format!("{path}.bim:material"))?
            .set_targets(["/World/Materials/Synthetic"])?;
        stage.prim(path)?.set_metadata(
            "customData",
            sdf::Value::Dictionary(
                [(
                    "future:note".to_owned(),
                    sdf::Value::String("untouched".to_owned()),
                )]
                .into(),
            ),
        )?;
    }
    stage.prim(OTHER_PATH)?.set_active(false)?;
    let text = stage.root_layer().export_to_string()?;
    let mut data = usda::parse(&text)?;
    use sdf::AbstractData;
    let root = sdf::path("/")?;
    data.set_field(&root, "metersPerUnit", sdf::Value::Double(0.001));
    data.set_field(&root, "upAxis", sdf::Value::token("Z"));
    data.set_field(
        &root,
        "customLayerData",
        sdf::Value::Dictionary([("future:layer".to_owned(), sdf::Value::Int(73))].into()),
    );
    usda::TextWriter::write_to_file(&data, destination)?;
    Ok(())
}

/// Original bounded structural check, not a USD grammar/conformance validator.
pub fn structural_check(text: &str) -> Result<()> {
    if text.len() > 16 * 1024 * 1024 || !text.starts_with("#usda 1.0") {
        return Err(failure("structural header/size"));
    }
    let mut delimiters = Vec::new();
    let (mut quoted, mut escaped, mut comment, mut asset) = (false, false, false, false);
    for c in text.chars() {
        if comment {
            comment = c != '\n';
        } else if quoted {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
        } else if asset {
            asset = c != '@';
        } else {
            match c {
                '#' => comment = true,
                '"' => quoted = true,
                '@' => asset = true,
                '{' | '(' | '[' => {
                    if delimiters.len() >= 64 {
                        return Err(failure("structural depth"));
                    }
                    delimiters.push(c);
                }
                '}' | ')' | ']' => {
                    let expected = match c {
                        '}' => '{',
                        ')' => '(',
                        _ => '[',
                    };
                    if delimiters.pop() != Some(expected) {
                        return Err(failure("structural delimiter"));
                    }
                }
                _ => {}
            }
        }
    }
    if quoted || asset || !delimiters.is_empty() {
        return Err(failure("structural truncation"));
    }
    Ok(())
}

/// Owned normalized key for the narrow static-field delta experiment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldKey {
    /// Full validated element identity.
    pub id: String,
    /// Authored attribute name; not a semantic footprint group.
    pub attribute: String,
}

/// Extract narrow same-namespace attribute keys from already translated edits.
pub fn normalized_keys_for_edits(
    paths_and_fields: &[(String, String)],
    identities: &BTreeMap<String, String>,
) -> Result<BTreeSet<FieldKey>> {
    if paths_and_fields.len() > 4096 {
        return Err(failure("LIMIT_EXCEEDED: delta"));
    }
    let mut keys = BTreeSet::new();
    for (path, field) in paths_and_fields {
        if field != "default" {
            return Err(failure("unsupported delta field"));
        }
        let (prim, attribute) = path
            .rsplit_once('.')
            .ok_or_else(|| failure("not an attribute"))?;
        let id = identities
            .get(prim)
            .ok_or_else(|| failure("unknown delta ownership"))?;
        if element_path(id)? != prim {
            return Err(failure("delta identity mismatch"));
        }
        keys.insert(FieldKey {
            id: id.clone(),
            attribute: attribute.to_owned(),
        });
    }
    Ok(keys)
}

// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! View forms and the validated CLI wire boundary; no model authoring.

use crate::tools::{ToolError, decode, rejection};
use beamcraft_engine::{
    RenderMode,
    model_view::{ModelView, Projection},
    work_plane::WorkPlane,
};
use beamcraft_json::Json;

/// The three independent browser categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrowserTab {
    /// Named and open model views.
    #[default]
    Views,
    /// Drawing documents.
    Drawings,
    /// Schedules.
    Schedules,
}

/// A supported view construction workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Construction {
    /// World XY plane at a height.
    #[default]
    XY,
    /// World XZ plane.
    XZ,
    /// World YZ plane.
    YZ,
    /// A vertical plane through two points.
    TwoPoints,
    /// Plane through three noncollinear points.
    ThreePoints,
    /// Active input frame.
    WorkPlane,
    /// One or many straight grid axes.
    Grid,
    /// A model level.
    Level,
    /// A picked planar face.
    Face,
    /// Pick a face, then a point on an edge for its X direction.
    FaceEdge,
    /// Four orthogonal/spatial views of a selected part.
    PartViews,
    /// A spatial view framed around a selected part.
    Part3D,
    /// A selected part's front plane.
    PartFront,
    /// A selected part's back plane.
    PartBack,
    /// A selected part's top plane.
    PartTop,
    /// An explicit plane frame.
    Explicit,
    /// Four views of the selected part's owning assembly.
    AssemblyViews,
    /// Spatial view of the owning assembly.
    Assembly3D,
    /// An undeformed straight-member representation.
    Undeformed,
    /// A selected part's bottom plane.
    PartBottom,
}
impl Construction {
    /// Available local creation workflows and their UI names.
    pub const ALL: [(Self, &'static str); 20] = [
        (Self::XY, "Plan (XY)"),
        (Self::XZ, "Elevation (XZ)"),
        (Self::YZ, "Elevation (YZ)"),
        (Self::TwoPoints, "Two points"),
        (Self::ThreePoints, "Three points"),
        (Self::WorkPlane, "Work plane"),
        (Self::Grid, "Grid axes"),
        (Self::Level, "Level"),
        (Self::Face, "Surface"),
        (Self::FaceEdge, "Surface and edge"),
        (Self::PartViews, "Part: four views"),
        (Self::Part3D, "Part: 3D"),
        (Self::PartFront, "Part: front"),
        (Self::PartBack, "Part: back"),
        (Self::PartTop, "Part: top"),
        (Self::PartBottom, "Part: bottom"),
        (Self::AssemblyViews, "Assembly: four views"),
        (Self::Assembly3D, "Assembly: 3D"),
        (Self::Undeformed, "Undeformed member"),
        (Self::Explicit, "Custom plane"),
    ];
    /// Whether invoking this command immediately requests geometry in the viewport.
    pub const fn picks_geometry(self) -> bool {
        matches!(
            self,
            Self::TwoPoints | Self::ThreePoints | Self::Face | Self::FaceEdge
        )
    }
    /// User-facing mode name.
    pub fn label(self) -> &'static str {
        Self::ALL
            .into_iter()
            .find(|(m, _)| *m == self)
            .map_or("Part: bottom", |(_, name)| name)
    }
}

/// Editable, unapplied view properties.
#[derive(Debug, Clone)]
pub struct Form {
    /// Existing view identity, or None for a new view.
    pub id: Option<String>,
    /// Name.
    pub name: String,
    /// Creation mode.
    pub construction: Construction,
    /// XYZ coordinate expressions for the construction points.
    pub points: [String; 3],
    /// Axis offset.
    pub offset: String,
    /// Grid/level selector.
    pub selector: String,
    /// Grid label, or batch mode.
    pub axis: String,
    /// Grid batch mode, or single axis.
    pub batch: String,
    /// Independent U/V axis selection modes.
    pub families: [String; 2],
    /// U/V name prefixes.
    pub prefixes: [String; 2],
    /// Existing view identities whose display settings are reused by each family.
    pub property_sources: [String; 2],
    /// Admitted explicit plane expression (editing an existing view).
    pub plane: String,
    /// Projection.
    pub projection: Projection,
    /// Vertical span in millimetres.
    pub span: String,
    /// Enable depth clipping.
    pub clipped: bool,
    /// Above and below distances.
    pub depth: [String; 2],
    /// Drawing style.
    pub mode: RenderMode,
    /// Grid display.
    pub grids: bool,
    /// Browser folder.
    pub folder: String,
    /// Quick-access membership.
    pub favorite: bool,
    /// Session-only view.
    pub temporary: bool,
    /// Hidden element-kind tokens.
    pub hidden: String,
    /// World bounds or `none`.
    pub area: String,
    /// Camera rotations, Z and X, in display degrees.
    pub rotation: [String; 2],
    /// Use a view-specific colour override.
    pub override_color: bool,
    /// Display colour (RGB).
    pub color: [f32; 3],
    /// Opacity.
    pub opacity: f32,
}
impl Default for Form {
    fn default() -> Self {
        Self {
            id: None,
            name: "New view".into(),
            construction: Construction::XY,
            points: ["0,0,0".into(), "1000,0,0".into(), "0,1000,0".into()],
            offset: "0".into(),
            selector: String::new(),
            axis: String::new(),
            batch: "families".into(),
            families: ["all".into(), "all".into()],
            prefixes: ["U".into(), "V".into()],
            property_sources: [String::new(), String::new()],
            plane: WorkPlane::default().expression(),
            projection: Projection::Orthographic,
            span: "10000".into(),
            clipped: false,
            depth: ["1000".into(), "1000".into()],
            mode: RenderMode::Shaded,
            grids: true,
            folder: String::new(),
            favorite: false,
            temporary: false,
            hidden: String::new(),
            area: "none".into(),
            rotation: ["0".into(), "0".into()],
            override_color: false,
            color: [0.65, 0.75, 0.8],
            opacity: 1.,
        }
    }
}
impl Form {
    /// Fill fields from a persisted view.
    pub fn from_view(view: &ModelView) -> Self {
        Self {
            id: Some(view.id.to_string()),
            name: view.name.clone(),
            plane: view.plane.expression(),
            projection: view.projection,
            span: view.span_mm.to_string(),
            clipped: view.depth.is_some(),
            depth: view.depth.unwrap_or([1000., 1000.]).map(|v| v.to_string()),
            mode: view.render_mode,
            grids: view.grids_visible,
            folder: view.folder.clone(),
            favorite: view.favorite,
            temporary: view.temporary,
            hidden: view
                .hidden_kinds
                .hidden_kinds()
                .iter()
                .map(|k| k.as_token())
                .collect::<Vec<_>>()
                .join(","),
            area: view.work_area.map_or("none".into(), |b| {
                b.min
                    .into_iter()
                    .chain(b.max)
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            }),
            rotation: view.rotation_rad.map(|v| v.to_degrees().to_string()),
            override_color: view.color.is_some(),
            color: view.color.unwrap_or([0.65, 0.75, 0.8]).map(|v| v as f32),
            opacity: view.opacity as f32,
            ..Self::default()
        }
    }
    /// Reuse a saved view's display properties while retaining the new view's construction.
    pub fn apply_display_properties(&mut self, view: &ModelView) {
        let source = Self::from_view(view);
        self.projection = source.projection;
        self.span = source.span;
        self.clipped = source.clipped;
        self.depth = source.depth;
        self.mode = source.mode;
        self.grids = source.grids;
        self.hidden = source.hidden;
        self.area = source.area;
        self.rotation = source.rotation;
        self.override_color = source.override_color;
        self.color = source.color;
        self.opacity = source.opacity;
    }
    /// Property arguments shared by create and update.
    pub fn property_args(&self) -> Vec<String> {
        vec![
            "--name".into(),
            self.name.trim().into(),
            "--projection".into(),
            self.projection.as_str().into(),
            "--span".into(),
            self.span.clone(),
            "--depth".into(),
            if self.clipped {
                self.depth.join(",")
            } else {
                "none".into()
            },
            "--render-mode".into(),
            self.mode.as_token().into(),
            "--grids".into(),
            self.grids.to_string(),
            "--folder".into(),
            self.folder.clone(),
            "--favorite".into(),
            self.favorite.to_string(),
            "--temporary".into(),
            self.temporary.to_string(),
            "--hidden-kinds".into(),
            self.hidden.clone(),
            "--work-area".into(),
            self.area.clone(),
            "--rotation".into(),
            self.rotation.join(","),
            "--color".into(),
            if self.override_color {
                self.color
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            } else {
                "default".into()
            },
            "--opacity".into(),
            self.opacity.to_string(),
        ]
    }
}

/// Decode every view before replacing the browser collection; malformed replies keep old state.
#[derive(Debug)]
/// A complete local collection returned by the view CLI.
pub struct Collection {
    /// Named and temporary views returned by this command.
    pub views: Vec<ModelView>,
    /// Browser folders, including empty folders.
    pub folders: Vec<String>,
}
/// Parse a validated collection and its browser folders.
pub fn parse_reply(code: Option<i32>, bytes: &[u8]) -> Result<Collection, ToolError> {
    let json = decode(bytes)?;
    if let Some(error) = rejection(&json, code) {
        return Err(error);
    }
    let bad = || ToolError::local("UNREADABLE_REPLY", "The view collection reply is invalid.");
    let array = json.get("views").and_then(Json::array).map_err(|_| bad())?;
    if array.len() > beamcraft_engine::model_view::MAX_VIEWS {
        return Err(bad());
    }
    let mut views = Vec::new();
    for value in array {
        let text = |key| value.get(key).and_then(Json::string).map_err(|_| bad());
        let depth = match value.get("depth_mm").map_err(|_| bad())? {
            Json::Null => None,
            json => {
                let [a, b] = json.array().map_err(|_| bad())? else {
                    return Err(bad());
                };
                Some([a.float().map_err(|_| bad())?, b.float().map_err(|_| bad())?])
            }
        };
        let [z, x] = value
            .get("rotation_rad")
            .and_then(Json::array)
            .map_err(|_| bad())?
        else {
            return Err(bad());
        };
        let color = match value.get("color").map_err(|_| bad())? {
            Json::Null => None,
            v => {
                let [r, g, b] = v.array().map_err(|_| bad())? else {
                    return Err(bad());
                };
                Some([
                    r.float().map_err(|_| bad())?,
                    g.float().map_err(|_| bad())?,
                    b.float().map_err(|_| bad())?,
                ])
            }
        };
        views.push(ModelView {
            id: beamcraft_model::id::Uuid7::parse(text("id")?).map_err(|_| bad())?,
            name: text("name")?.into(),
            plane: WorkPlane::parse(text("plane")?).map_err(|_| bad())?,
            projection: Projection::parse(text("projection")?).map_err(|_| bad())?,
            span_mm: value
                .get("span_mm")
                .and_then(Json::float)
                .map_err(|_| bad())?,
            depth,
            render_mode: RenderMode::parse(text("render_mode")?).ok_or_else(bad)?,
            grids_visible: value
                .get("grids_visible")
                .and_then(Json::boolean)
                .map_err(|_| bad())?,
            rotation_rad: [z.float().map_err(|_| bad())?, x.float().map_err(|_| bad())?],
            color,
            opacity: value
                .get("opacity")
                .and_then(Json::float)
                .map_err(|_| bad())?,
            folder: text("folder")?.into(),
            favorite: value
                .get("favorite")
                .and_then(Json::boolean)
                .map_err(|_| bad())?,
            temporary: value
                .get("temporary")
                .and_then(Json::boolean)
                .map_err(|_| bad())?,
            hidden_kinds: beamcraft_engine::model_view::hidden_kinds(text("hidden_kinds")?)
                .map_err(|_| bad())?,
            work_area: match value.get("work_area").map_err(|_| bad())? {
                Json::Null => None,
                area => {
                    let v = area
                        .array()
                        .map_err(|_| bad())?
                        .iter()
                        .map(Json::float)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| bad())?;
                    let [a, b, c, d, e, f] = v.as_slice() else {
                        return Err(bad());
                    };
                    Some(beamcraft_engine::Aabb {
                        min: [*a, *b, *c],
                        max: [*d, *e, *f],
                    })
                }
            },
        });
    }
    beamcraft_engine::model_view::validate_collection(&views).map_err(|_| bad())?;
    let folders = json
        .get("folders")
        .and_then(Json::array)
        .map_err(|_| bad())?;
    if folders.len() > 512 {
        return Err(bad());
    }
    let folders = folders
        .iter()
        .map(|v| v.string().map(str::to_owned).map_err(|_| bad()))
        .collect::<Result<Vec<_>, _>>()?;
    for folder in &folders {
        beamcraft_engine::model_view::validate_folder(folder).map_err(|_| bad())?;
    }
    Ok(Collection { views, folders })
}

// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Named model-view definitions. Presentation state never changes a model or snapshot token.

use crate::work_plane::WorkPlane;
use beamcraft_model::{command::CommandError, error::ErrorCode, grid::Grid};

/// Maximum named views in one local view collection.
pub const MAX_VIEWS: usize = 256;

/// One selected straight grid axis and the plane through it.
pub struct GridViewAxis {
    /// U is zero; V is one.
    pub family: u8,
    /// Original axis label.
    pub label: String,
    /// Vertical view plane.
    pub plane: WorkPlane,
}

/// Plan independent U/V batches: none, first (nearest the local origin), last, or all.
pub fn grid_views(grid: &Grid, u: &str, v: &str) -> Result<Vec<GridViewAxis>, CommandError> {
    let lines = crate::grid_query::segments(grid)?;
    let mut result = Vec::new();
    for (family, mode) in [(0, u), (1, v)] {
        let mut selected: Vec<_> = lines.iter().filter(|l| l.family == family).collect();
        let distance = |line: &&crate::grid_query::GridSegment| {
            let [x, y] = line.start;
            let [ex, ey] = line.end;
            (x * (ey - y) - y * (ex - x)).abs() / (ex - x).hypot(ey - y)
        };
        match mode {
            "none" => selected.clear(),
            "all" => {}
            "first" | "last" => {
                selected.sort_by(|a, b| {
                    distance(a)
                        .total_cmp(&distance(b))
                        .then_with(|| a.axis_id.cmp(&b.axis_id))
                });
                if mode == "last" {
                    selected.reverse();
                }
                selected.truncate(1);
            }
            _ => return Err(invalid("grid family mode must be none, first, last or all")),
        }
        for line in selected {
            result.push(GridViewAxis {
                family,
                label: line.label.clone(),
                plane: grid_line_plane(grid, line)?,
            });
        }
    }
    Ok(result)
}

/// Decode a view's hidden-kind list; empty means every kind is visible.
pub fn hidden_kinds(text: &str) -> Result<crate::KindFilter, crate::ViewError> {
    if text.is_empty() {
        Ok(crate::KindFilter::SHOW_ALL)
    } else {
        crate::KindFilter::parse_list(text)
    }
}

/// Validate a browser-only folder path without treating it as a filesystem path.
pub fn validate_folder(folder: &str) -> Result<(), CommandError> {
    if folder.len() > 512
        || folder.chars().any(char::is_control)
        || folder.split('/').count() > 8
        || (!folder.is_empty()
            && folder
                .split('/')
                .any(|s| s.trim() != s || s.is_empty() || matches!(s, "." | "..")))
    {
        return Err(invalid("folder needs at most eight nonempty path segments"));
    }
    Ok(())
}

/// Camera projection of a model view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Projection {
    /// Parallel rays and constant scale with depth.
    #[default]
    Orthographic,
    /// A 45-degree vertical perspective field of view.
    Perspective,
}

impl Projection {
    /// Stable wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Orthographic => "orthographic",
            Self::Perspective => "perspective",
        }
    }
    /// Parse a closed projection tag.
    pub fn parse(text: &str) -> Result<Self, CommandError> {
        match text {
            "orthographic" => Ok(Self::Orthographic),
            "perspective" => Ok(Self::Perspective),
            _ => Err(invalid("projection must be orthographic or perspective")),
        }
    }
}

/// Properties shared by plane and spatial views, in model millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelView {
    /// Persistent UUIDv7 identity of this local view.
    pub id: beamcraft_model::id::Uuid7,
    /// User-visible name, unique within the collection.
    pub name: String,
    /// View plane and input frame. Local +Z points towards the viewer.
    pub plane: WorkPlane,
    /// Camera projection.
    pub projection: Projection,
    /// Vertical visible span at the view target.
    pub span_mm: f64,
    /// Visible distance from the plane towards the viewer; None disables slab clipping.
    pub depth: Option<[f64; 2]>,
    /// Display mode, independent for each view.
    pub render_mode: crate::RenderMode,
    /// Whether grid guides are displayed in this view.
    pub grids_visible: bool,
    /// Browser folder path, separated by `/`; empty is the root folder.
    pub folder: String,
    /// Include this view in quick access.
    pub favorite: bool,
    /// Session-only views are deliberately omitted from saved collections.
    pub temporary: bool,
    /// Kinds hidden independently in this view.
    pub hidden_kinds: crate::KindFilter,
    /// Optional world-space display work area, independent of the engineering bounds.
    pub work_area: Option<crate::Aabb>,
    /// Camera rotations about the view's local Z, then X, in radians. The input plane stays fixed.
    pub rotation_rad: [f64; 2],
    /// Optional display colour override, linear RGB in 0..1.
    pub color: Option<[f64; 3]>,
    /// Display opacity in 0..1; engineering geometry is unchanged.
    pub opacity: f64,
}

fn invalid(message: &'static str) -> CommandError {
    CommandError::new(ErrorCode::InvalidArgument, "model_view", message)
}

impl ModelView {
    /// Reuse display settings without changing the recipient's identity, plane, name or folder.
    pub fn apply_display_properties(&mut self, source: &Self) {
        self.projection = source.projection;
        self.span_mm = source.span_mm;
        self.depth = source.depth;
        self.render_mode = source.render_mode;
        self.grids_visible = source.grids_visible;
        self.hidden_kinds = source.hidden_kinds;
        self.work_area = source.work_area;
        self.rotation_rad = source.rotation_rad;
        self.color = source.color;
        self.opacity = source.opacity;
    }
    /// Reject malformed names, nonfinite sizes and out-of-range clipping distances.
    pub fn validate(&self) -> Result<(), CommandError> {
        if self.name.trim() != self.name
            || self.name.is_empty()
            || self.name.chars().count() > 128
            || self.name.chars().any(char::is_control)
        {
            return Err(invalid(
                "view name needs 1 to 128 characters without outer spaces or controls",
            ));
        }
        if !self.span_mm.is_finite() || !(1.0..=20_000_000.).contains(&self.span_mm) {
            return Err(invalid("view span must be between 1 and 20000000 mm"));
        }
        if self
            .rotation_rad
            .iter()
            .any(|v| !v.is_finite() || v.abs() > std::f64::consts::TAU * 1000.)
            || !self.opacity.is_finite()
            || !(0.0..=1.0).contains(&self.opacity)
            || self
                .color
                .is_some_and(|c| c.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)))
        {
            return Err(invalid(
                "camera rotation, colour or opacity is out of range",
            ));
        }
        validate_folder(&self.folder)?;
        if let Some(area) = self.work_area {
            let [ax, ay, az] = area.min;
            let [bx, by, bz] = area.max;
            if area
                .min
                .into_iter()
                .chain(area.max)
                .any(|v| !v.is_finite() || v.abs() > beamcraft_model::limits::COORDINATE_CAP_MM)
                || ax > bx
                || ay > by
                || az > bz
            {
                return Err(invalid(
                    "work area must contain finite ordered model bounds",
                ));
            }
        }
        if let Some([above, below]) = self.depth
            && (!above.is_finite()
                || !below.is_finite()
                || !(0.0..=20_000_000.).contains(&above)
                || !(0.0..=20_000_000.).contains(&below)
                || above + below <= 0.)
        {
            return Err(invalid(
                "view depth needs bounded nonnegative distances and positive total thickness",
            ));
        }
        Ok(())
    }

    /// Whether a world point lies in the depth slab. Used for visible-only picking as well as display.
    pub fn contains(&self, point: [f64; 3]) -> bool {
        if self.opacity == 0. || point.iter().any(|v| !v.is_finite()) {
            return false;
        }
        if let Some(area) = self.work_area
            && point
                .into_iter()
                .zip(area.min.into_iter().zip(area.max))
                .any(|(v, (lo, hi))| v < lo || v > hi)
        {
            return false;
        }
        let Some([above, below]) = self.depth else {
            return true;
        };
        let [x, y, z] = point;
        let [ox, oy, oz] = self.plane.origin();
        let [nx, ny, nz] = self.plane.normal();
        let d = (x - ox) * nx + (y - oy) * ny + (z - oz) * nz;
        d >= -below && d <= above
    }

    /// Camera frame derived from the persistent view plane and the explicit camera rotations.
    pub fn camera_plane(&self) -> Result<WorkPlane, CommandError> {
        self.validate()?;
        let [z, x] = self.rotation_rad;
        let (sz, cz) = z.sin_cos();
        let (sx, cx) = x.sin_cos();
        let right: [f64; 3] = std::array::from_fn(|i| {
            self.plane.x().get(i).copied().unwrap_or(0.) * cz
                + self.plane.y().get(i).copied().unwrap_or(0.) * sz
        });
        let up: [f64; 3] = std::array::from_fn(|i| {
            (-self.plane.x().get(i).copied().unwrap_or(0.) * sz
                + self.plane.y().get(i).copied().unwrap_or(0.) * cz)
                * cx
                + self.plane.normal().get(i).copied().unwrap_or(0.) * sx
        });
        WorkPlane::new(self.plane.origin(), right, up)
    }
}

/// View a part along its local frame, with framing determined by its evaluated geometry.
pub fn part_plane(
    snapshot: &crate::ViewSnapshot,
    id: &crate::ElementId,
    side: &str,
) -> Result<(WorkPlane, f64), CommandError> {
    let entry = snapshot.element(id).ok_or_else(|| {
        CommandError::new(ErrorCode::NotFound, "element", "view target was not found")
    })?;
    let bounds = entry.full.world_bounds;
    let [ax, ay, az] = bounds.min;
    let [bx, by, bz] = bounds.max;
    let origin = [(ax + bx) / 2., (ay + by) / 2., (az + bz) / 2.];
    let (x, y) = match side {
        "front" => ([1., 0., 0.], [0., 0., 1.]),
        "back" => ([-1., 0., 0.], [0., 0., 1.]),
        "top" => ([1., 0., 0.], [0., 1., 0.]),
        "bottom" => ([1., 0., 0.], [0., -1., 0.]),
        "end" => ([0., 1., 0.], [0., 0., 1.]),
        "3d" => {
            let (s, c) = (-0.8f64).sin_cos();
            let (sp, cp) = 0.85f64.sin_cos();
            ([-s, c, 0.], [-c * sp, -s * sp, cp])
        }
        _ => return Err(invalid("unknown part viewing direction")),
    };
    let rotate = |v| crate::query::snapshot::rotate_vector(v, entry.orientation_xyzw);
    let plane = WorkPlane::new(origin, rotate(x), rotate(y))?;
    let span = ((bx - ax).hypot(by - ay).hypot(bz - az) * 1.25).clamp(1., 20_000_000.);
    Ok((plane, span))
}

/// Frame a verified collection of parts using one owning part's local directions.
pub fn group_plane(
    snapshot: &crate::ViewSnapshot,
    parts: &[crate::ElementId],
    side: &str,
) -> Result<(WorkPlane, f64), CommandError> {
    if parts.is_empty() || parts.len() > 65_536 {
        return Err(invalid("view group must contain 1 to 65536 parts"));
    }
    let owner = parts
        .first()
        .ok_or_else(|| invalid("view group is empty"))?;
    let (plane, _) = part_plane(snapshot, owner, side)?;
    let mut bounds = None;
    for id in parts {
        let next = snapshot
            .element_bounds(id)
            .map_err(|_| invalid("group part geometry is unavailable"))?;
        bounds = Some(bounds.map_or(next, |old: crate::Aabb| old.union(next)));
    }
    let bounds = bounds.ok_or_else(|| invalid("view group is empty"))?;
    let [ax, ay, az] = bounds.min;
    let [bx, by, bz] = bounds.max;
    Ok((
        WorkPlane::new(
            [(ax + bx) / 2., (ay + by) / 2., (az + bz) / 2.],
            plane.x(),
            plane.y(),
        )?,
        ((bx - ax).hypot(by - ay).hypot(bz - az) * 1.25).clamp(1., 20_000_000.),
    ))
}

/// Surface view frame, optionally aligned with a chosen in-plane edge.
pub fn surface_plane(
    snapshot: &crate::ViewSnapshot,
    origin: [f64; 3],
    direction: [f64; 3],
    edge: Option<[[f64; 3]; 2]>,
) -> Result<WorkPlane, CommandError> {
    surface_plane_in_view(snapshot, None, origin, direction, edge)
}

/// Resolve a surface using the same depth and kind visibility as the viewport.
pub fn surface_plane_in_view(
    snapshot: &crate::ViewSnapshot,
    view: Option<&ModelView>,
    origin: [f64; 3],
    direction: [f64; 3],
    edge: Option<[[f64; 3]; 2]>,
) -> Result<WorkPlane, CommandError> {
    use crate::query::{
        feature::{FeatureGeometry, FeatureKind, lookup_feature},
        pick::{PickQuery, RayPick, SelectionFilter, pick},
    };
    let hit = if let Some(view) = view {
        pick_visible(snapshot, view, origin, direction, SelectionFilter::ALL)?
    } else {
        let mut query = RayPick::new(origin, direction, vec![FeatureKind::Face]);
        query.max_hits = 1;
        pick(snapshot, &PickQuery::Ray(query))
            .map_err(|_| invalid("surface pick failed"))?
            .hits
            .into_iter()
            .next()
    }
    .ok_or_else(|| {
        CommandError::new(
            ErrorCode::NotFound,
            "face",
            "no visible planar face was hit",
        )
    })?;
    let feature = hit
        .feature_id
        .as_ref()
        .ok_or_else(|| invalid("surface has no planar feature"))?;
    let feature = lookup_feature(snapshot, &feature.to_string())
        .map_err(|_| invalid("surface feature is unavailable"))?;
    let normal = match feature.world {
        FeatureGeometry::Face { normal, .. }
        | FeatureGeometry::Surface {
            normal: Some(normal),
            ..
        } => normal,
        _ => {
            return Err(CommandError::new(
                ErrorCode::UnsupportedGeometry,
                "face",
                "view requires a supported planar face",
            ));
        }
    };
    let plane = WorkPlane::face(hit.point_world_mm, normal)?;
    let automatic = if edge.is_none() {
        let mut longest = None;
        {
            let [px, py, pz] = plane.origin();
            let [nx, ny, nz] = plane.normal();
            let on_plane =
                |[x, y, z]: [f64; 3]| ((x - px) * nx + (y - py) * ny + (z - pz) * nz).abs() < 1e-5;
            let edges = crate::query::feature::element_edges(snapshot, &hit.element_id)
                .map_err(|_| invalid("surface boundary is unavailable"))?;
            let length = |[[ax, ay, az], [bx, by, bz]]: [[f64; 3]; 2]| {
                (bx - ax).hypot(by - ay).hypot(bz - az)
            };
            for candidate in edges
                .into_iter()
                .filter(|[a, b]| on_plane(*a) && on_plane(*b))
            {
                if longest.is_none_or(|old| length(candidate) > length(old)) {
                    longest = Some(candidate);
                }
            }
        }
        longest
    } else {
        None
    };
    let Some([a, b]) = edge.or(automatic) else {
        return Ok(plane);
    };
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    let [nx, ny, nz] = plane.normal();
    let [px, py, pz] = plane.origin();
    let on_plane =
        |[x, y, z]: [f64; 3]| ((x - px) * nx + (y - py) * ny + (z - pz) * nz).abs() < 1e-5;
    if !on_plane(a) || !on_plane(b) {
        return Err(invalid("the chosen edge must lie on the picked plane"));
    }
    let x = [bx - ax, by - ay, bz - az];
    let [xx, xy, xz] = x;
    WorkPlane::new(
        a,
        x,
        [ny * xz - nz * xy, nz * xx - nx * xz, nx * xy - ny * xx],
    )
}

/// Validate the complete collection before persistence, including identities and names.
pub fn validate_collection(views: &[ModelView]) -> Result<(), CommandError> {
    if views.len() > MAX_VIEWS {
        return Err(CommandError::new(
            ErrorCode::LimitExceeded,
            "views",
            "view collection exceeds 256 views",
        ));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut ids = std::collections::BTreeSet::new();
    for view in views {
        view.validate()?;
        if !names.insert(view.name.to_lowercase()) || !ids.insert(view.id) {
            return Err(CommandError::new(
                ErrorCode::DuplicateId,
                "views",
                "view names and identities must be unique",
            ));
        }
    }
    Ok(())
}

/// A vertical plane through two spatial points, with local Y up along world Z.
pub fn two_points(a: [f64; 3], b: [f64; 3]) -> Result<WorkPlane, CommandError> {
    WorkPlane::vertical(a, b)
}

/// A section through two points perpendicular to the active input plane.
pub fn two_points_in_plane(
    a: [f64; 3],
    b: [f64; 3],
    input: WorkPlane,
) -> Result<WorkPlane, CommandError> {
    // Validate the original endpoints before projecting the direction onto the input plane.
    WorkPlane::new(a, [1., 0., 0.], [0., 1., 0.])?;
    WorkPlane::new(b, [1., 0., 0.], [0., 1., 0.])?;
    let [ax, ay, az] = a;
    let [bx, by, bz] = b;
    let [nx, ny, nz] = input.normal();
    let d = (bx - ax) * nx + (by - ay) * ny + (bz - az) * nz;
    WorkPlane::new(
        a,
        [bx - ax - d * nx, by - ay - d * ny, bz - az - d * nz],
        [nx, ny, nz],
    )
}

/// A plane through three points: origin, +X point and positive-Y side point.
pub fn three_points(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Result<WorkPlane, CommandError> {
    WorkPlane::three_points(a, b, c)
}

/// A vertical plane along the selected straight grid axis, including rotated/drawn grids.
pub fn grid_axis(grid: &Grid, label: &str) -> Result<WorkPlane, CommandError> {
    let lines = crate::grid_query::segments(grid)?;
    let mut matching = lines
        .iter()
        .filter(|line| line.label == label || line.axis_id.to_string() == label);
    let line = matching
        .next()
        .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "axis", "grid axis was not found"))?;
    if matching.next().is_some() {
        return Err(invalid("axis label is ambiguous; use its identity"));
    }
    grid_line_plane(grid, line)
}
fn grid_line_plane(
    grid: &Grid,
    line: &crate::grid_query::GridSegment,
) -> Result<WorkPlane, CommandError> {
    let [ax, ay] = crate::grid_query::local_to_world(grid, line.start);
    let [bx, by] = crate::grid_query::local_to_world(grid, line.end);
    two_points([ax, ay, 0.], [bx, by, 0.])
}

/// Pick only geometry visible in this view. Clipped/filtered front objects cannot mask a visible hit.
pub fn pick_visible(
    snapshot: &crate::ViewSnapshot,
    view: &ModelView,
    origin: [f64; 3],
    direction: [f64; 3],
    selection: crate::query::pick::SelectionFilter,
) -> Result<Option<crate::query::pick::Hit>, CommandError> {
    use crate::query::{
        feature::FeatureKind,
        pick::{MAX_HITS, PickQuery, RayPick, pick},
    };
    view.validate()?;
    let mut query = RayPick::new(origin, direction, vec![FeatureKind::Face]);
    query.max_hits = MAX_HITS;
    query.selection = selection;
    let hits = pick(snapshot, &PickQuery::Ray(query))
        .map_err(|e| CommandError::new(e.f04_code(), "view.pick", "visible view pick failed"))?;
    if let Some(hit) = hits.hits.into_iter().find(|hit| {
        view.contains(hit.point_world_mm)
            && snapshot
                .element(&hit.element_id)
                .is_some_and(|e| view.hidden_kinds.is_visible(e.kind))
    }) {
        return Ok(Some(hit));
    }
    if hits.truncated {
        return Err(CommandError::new(
            ErrorCode::LimitExceeded,
            "view.pick",
            "visible view pick exceeds the hit budget",
        ));
    }
    Ok(None)
}

// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
//! Orbit and plane cameras: orbit, pan and zoom with perspective or orthographic projection,
//! right-handed Z up, millimetres. Display only; derived from the V01 spike.
use crate::Error;
/// Bounded orbit/pan/zoom state; fields private to preserve finiteness.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    target: [f32; 3],
    yaw: f32,
    pitch: f32,
    distance: f32,
    half_span: f32,
    basis: Option<[[f32; 3]; 3]>,
    projection: beamcraft_engine::model_view::Projection,
    clip_plane: [f32; 4],
    clip_depth: [f32; 4],
    area_min: [f32; 4],
    area_max: [f32; 4],
    color: [f32; 4],
}
/// Validated camera basis; four-component vectors match WGSL uniform alignment.
#[derive(Debug, Clone, Copy)]
pub struct View {
    /// Screen right and padding.
    pub right: [f32; 4],
    /// Screen up and padding.
    pub up: [f32; 4],
    /// From target to eye and padding.
    pub back: [f32; 4],
    /// Eye position and padding.
    pub eye: [f32; 4],
    /// tan(vertical FOV/2), aspect, near and far.
    pub params: [f32; 4],
    /// Orthographic flag and half vertical span (otherwise perspective).
    pub projection: [f32; 4],
    /// Depth slab normal and plane offset in scene-relative coordinates.
    pub clip_plane: [f32; 4],
    /// Above, below, enabled, padding.
    pub clip_depth: [f32; 4],
    /// World work-area minimum relative to scene origin, with enabled flag in W.
    pub area_min: [f32; 4],
    /// World work-area maximum relative to scene origin.
    pub area_max: [f32; 4],
    /// Optional colour override (negative R uses the default), with display opacity.
    pub color: [f32; 4],
}
fn dot([x, y, z]: [f32; 3], [a, b, c]: [f32; 3]) -> f32 {
    x * a + y * b + z * c
}
impl Camera {
    /// Frames a bounded scene for a viewport of `aspect` (width / height): `extent` is the
    /// diagonal of its bounds in millimetres. The bounding sphere fits both the vertical and the
    /// horizontal field of view, so a portrait viewport is framed as well as a landscape one.
    pub fn new(target: [f32; 3], extent: f32, aspect: f32) -> Result<Self, Error> {
        if !target.iter().all(|x| x.is_finite() && x.abs() <= 1e7)
            || !extent.is_finite()
            || !(1.0..=1e7).contains(&extent)
            || !aspect.is_finite()
            || !(0.05..=20.0).contains(&aspect)
        {
            return Err(Error::Input("camera framing"));
        }
        let vertical = std::f32::consts::PI / 8.0;
        let horizontal = (vertical.tan() * aspect).atan();
        let half = vertical.min(horizontal);
        let distance = (extent / 2.0 / half.sin() * 1.05).clamp(10.0, 2e7);
        Ok(Self {
            target,
            yaw: -0.8,
            pitch: 0.85,
            distance,
            half_span: distance * vertical.tan(),
            basis: None,
            projection: beamcraft_engine::model_view::Projection::Perspective,
            clip_plane: [0.; 4],
            clip_depth: [0.; 4],
            area_min: [0.; 4],
            area_max: [0.; 4],
            color: [-1., 0., 0., 1.],
        })
    }
    /// Orbit (radians), pan (screen-relative mm) and logarithmic zoom; reject hostile inputs.
    pub fn navigate(&mut self, orbit: [f32; 2], pan: [f32; 2], zoom: f32) -> Result<(), Error> {
        if !orbit
            .into_iter()
            .chain(pan)
            .chain([zoom])
            .all(|x| x.is_finite() && x.abs() <= 1e6)
        {
            return Err(Error::Input("camera navigation"));
        }
        let [ox, oy] = orbit;
        let [px, py] = pan;
        if let Some([right, up, back]) = self.basis {
            let rotate = |v: [f32; 3], axis: [f32; 3], angle: f32| {
                let [x, y, z] = v;
                let [a, b, c] = axis;
                let cross = [b * z - c * y, c * x - a * z, a * y - b * x];
                let (s, co) = angle.sin_cos();
                let d = dot(v, axis);
                std::array::from_fn(|i| {
                    v.get(i).copied().unwrap_or(0.) * co
                        + cross.get(i).copied().unwrap_or(0.) * s
                        + axis.get(i).copied().unwrap_or(0.) * d * (1. - co)
                })
            };
            let right = rotate(right, up, ox);
            let back = rotate(back, up, ox);
            self.basis = Some([right, rotate(up, right, -oy), rotate(back, right, -oy)]);
        } else {
            self.yaw = (self.yaw + ox).rem_euclid(std::f32::consts::TAU);
            self.pitch = (self.pitch + oy).clamp(0.08, 1.52);
        }
        let scale = (-zoom.clamp(-10.0, 10.0)).exp();
        self.distance = (self.distance * scale).clamp(10.0, 2e7);
        self.half_span = (self.half_span * scale).clamp(0.5, 1e7);
        let [above, _, clipped, _] = self.clip_depth;
        if self.projection == beamcraft_engine::model_view::Projection::Orthographic
            && clipped > 0.5
        {
            self.distance = self.distance.max(above + 10.);
        }
        let view = self.view(1.0)?;
        let [rx, ry, rz, _] = view.right;
        let [ux, uy, uz, _] = view.up;
        let [x, y, z] = self.target;
        self.target = [
            (x + rx * px + ux * py).clamp(-1e7, 1e7),
            (y + ry * px + uy * py).clamp(-1e7, 1e7),
            (z + rz * px + uz * py).clamp(-1e7, 1e7),
        ];
        Ok(())
    }
    /// Frames new bounds for a viewport of `aspect` while keeping the viewing direction.
    pub fn fit(&mut self, target: [f32; 3], extent: f32, aspect: f32) -> Result<(), Error> {
        let framed = Self::new(target, extent, aspect)?;
        self.target = framed.target;
        self.distance = framed.distance;
        self.half_span = framed.half_span;
        Ok(())
    }
    /// Distance used to convert UI pan pixels to display millimetres.
    pub fn distance(&self) -> f32 {
        if self.projection == beamcraft_engine::model_view::Projection::Orthographic {
            self.half_span / (std::f32::consts::PI / 8.).tan()
        } else {
            self.distance
        }
    }
    /// Align the camera with a plane, retaining this view's clipping and appearance.
    pub fn aligned(
        &self,
        plane: beamcraft_engine::work_plane::WorkPlane,
        origin: [f64; 3],
        span: f64,
    ) -> Result<Self, Error> {
        let [x, y, z] = plane.origin();
        let [ox, oy, oz] = origin;
        let mut camera = *self;
        let target = [(x - ox) as f32, (y - oy) as f32, (z - oz) as f32];
        if !span.is_finite() || !(1.0..=20_000_000.).contains(&span) {
            return Err(Error::Input("view span"));
        }
        Self::new(target, 1000., 1.)?;
        camera.half_span = span as f32 / 2.;
        camera.target = target;
        camera.basis = Some([
            plane.x().map(|v| v as f32),
            plane.y().map(|v| v as f32),
            plane.normal().map(|v| v as f32),
        ]);
        camera.distance = (span as f32 / (2. * (std::f32::consts::PI / 8.).tan())).clamp(10., 2e7);
        camera.projection = beamcraft_engine::model_view::Projection::Orthographic;
        let [above, _, clipped, _] = camera.clip_depth;
        if clipped > 0.5 {
            camera.distance = camera.distance.max(above + 10.);
        }
        Ok(camera)
    }
    /// Preserve world-space framing after a snapshot changes its display origin.
    pub fn rebase_origin(&mut self, previous: [f64; 3], next: [f64; 3]) {
        let delta: [f32; 3] = std::array::from_fn(|i| {
            (previous.get(i).copied().unwrap_or(0.) - next.get(i).copied().unwrap_or(0.)) as f32
        });
        for (v, d) in self.target.iter_mut().zip(delta) {
            *v += d;
        }
        let [nx, ny, nz, offset] = self.clip_plane;
        self.clip_plane = [nx, ny, nz, offset + dot(delta, [nx, ny, nz])];
        for (v, d) in self.area_min.iter_mut().take(3).zip(delta) {
            *v += d;
        }
        for (v, d) in self.area_max.iter_mut().take(3).zip(delta) {
            *v += d;
        }
    }
    /// Restore a validated model view without changing the model or uploading geometry.
    pub fn from_model_view(
        view: &beamcraft_engine::model_view::ModelView,
        origin: [f64; 3],
    ) -> Result<Self, Error> {
        view.validate().map_err(|_| Error::Input("model view"))?;
        let [x, y, z] = view.plane.origin();
        let [ox, oy, oz] = origin;
        let target = [(x - ox) as f32, (y - oy) as f32, (z - oz) as f32];
        let mut camera = Self::new(target, 1000., 1.)?;
        let normal = view.plane.normal().map(|v| v as f32);
        let camera_plane = view
            .camera_plane()
            .map_err(|_| Error::Input("view rotation"))?;
        camera.basis = Some([
            camera_plane.x().map(|v| v as f32),
            camera_plane.y().map(|v| v as f32),
            camera_plane.normal().map(|v| v as f32),
        ]);
        camera.distance =
            (view.span_mm as f32 / (2. * (std::f32::consts::PI / 8.).tan())).clamp(10., 2e7);
        camera.projection = view.projection;
        camera.half_span = view.span_mm as f32 / 2.;
        let [r, g, b] = view.color.unwrap_or([-1., 0., 0.]);
        camera.color = [r as f32, g as f32, b as f32, view.opacity as f32];
        let [nx, ny, nz] = normal;
        camera.clip_plane = [nx, ny, nz, dot(target, normal)];
        if let Some([above, below]) = view.depth {
            camera.clip_depth = [above as f32, below as f32, 1., 0.];
            if view.projection == beamcraft_engine::model_view::Projection::Orthographic {
                camera.distance = camera.distance.max(above as f32 + 10.);
            }
        }
        if let Some(area) = view.work_area {
            let [a, b, c] = area.min;
            let [d, e, f] = area.max;
            camera.area_min = [(a - ox) as f32, (b - oy) as f32, (c - oz) as f32, 1.];
            camera.area_max = [(d - ox) as f32, (e - oy) as f32, (f - oz) as f32, 0.];
        }
        Ok(camera)
    }
    /// Current camera target frame and visible vertical span for explicit Save camera.
    pub fn model_frame(
        &self,
        origin: [f64; 3],
    ) -> Result<(beamcraft_engine::work_plane::WorkPlane, f64), Error> {
        let view = self.view(1.)?;
        let [x, y, z] = self.target;
        let [ox, oy, oz] = origin;
        let [rx, ry, rz, _] = view.right;
        let [ux, uy, uz, _] = view.up;
        let origin = [ox + f64::from(x), oy + f64::from(y), oz + f64::from(z)];
        let [a, b, c] = origin;
        let plane = beamcraft_engine::work_plane::WorkPlane::three_points(
            origin,
            [a + f64::from(rx), b + f64::from(ry), c + f64::from(rz)],
            [a + f64::from(ux), b + f64::from(uy), c + f64::from(uz)],
        )
        .map_err(|_| Error::Input("camera frame"))?;
        Ok((
            plane,
            f64::from(
                if self.projection == beamcraft_engine::model_view::Projection::Orthographic {
                    2. * self.half_span
                } else {
                    2. * self.distance * (std::f32::consts::PI / 8.).tan()
                },
            ),
        ))
    }
    /// The pan (millimetres) that keeps the point under the cursor fixed while zooming: `ndc` is
    /// the cursor in normalized device coordinates (`edit::ndc_of`, y up, inside `[-1, 1]`) and
    /// `zoom` the same logarithmic input `navigate` takes. On the target plane the view scales
    /// from the distance `navigate` will actually reach (including its clamps), so moving the
    /// target by `(1 - s)` of the cursor's offset from the target keeps the anchor still even at
    /// the zoom bounds; zero zoom gives zero pan.
    pub fn cursor_zoom_pan(
        &self,
        ndc: [f32; 2],
        aspect: f32,
        zoom: f32,
    ) -> Result<[f32; 2], Error> {
        if !ndc.into_iter().all(|x| x.is_finite())
            || ndc[0].abs() > 1.0
            || ndc[1].abs() > 1.0
            || !zoom.is_finite()
        {
            return Err(Error::Input("camera cursor zoom"));
        }
        let view = self.view(aspect)?;
        let [tan, aspect, _, _] = view.params;
        let orthographic =
            self.projection == beamcraft_engine::model_view::Projection::Orthographic;
        let d = if orthographic {
            self.half_span / tan
        } else {
            self.distance
        };
        let scale = (-zoom.clamp(-10.0, 10.0)).exp();
        let s = if orthographic {
            (self.half_span * scale).clamp(0.5, 1e7) / self.half_span
        } else {
            (d * scale).clamp(10.0, 2e7) / d
        };
        let nx = ndc[0].clamp(-1.0, 1.0);
        let ny = ndc[1].clamp(-1.0, 1.0);
        // Clamp each component into what navigate accepts: near the distance bounds the raw
        // compensation (up to d * tan * aspect) would get the whole frame rejected.
        Ok([
            ((1.0 - s) * d * nx * tan * aspect).clamp(-1e6, 1e6),
            ((1.0 - s) * d * ny * tan).clamp(-1e6, 1e6),
        ])
    }
    /// Perspective basis with depth range `[0, 1]`, vertical FOV 45 degrees.
    pub fn view(&self, aspect: f32) -> Result<View, Error> {
        if !aspect.is_finite() || !(0.05..=20.0).contains(&aspect) {
            return Err(Error::Input("camera aspect"));
        }
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let basis = self.basis.unwrap_or([
            [-sy, cy, 0.],
            [-cy * sp, -sy * sp, cp],
            [cy * cp, sy * cp, sp],
        ]);
        let [r, u, b] = basis;
        let pad = |[x, y, z]: [f32; 3]| [x, y, z, 0.];
        let back = pad(b);
        let right = pad(r);
        let up = pad(u);
        let [x, y, z] = self.target;
        let [bx, by, bz, _] = back;
        // The near plane follows the camera distance. A fixed 1 mm near plane leaves a
        // `Depth32Float` buffer too little resolution over a long model (far is at least
        // 1e7 mm), so hidden parts showed through; one percent of the distance keeps
        // precision where the orbit camera can be anyway.
        let near = if self.projection == beamcraft_engine::model_view::Projection::Orthographic {
            0.1
        } else {
            (self.distance * 0.01).clamp(0.1, 1e5)
        };
        Ok(View {
            right,
            up,
            back,
            eye: [
                x + bx * self.distance,
                y + by * self.distance,
                z + bz * self.distance,
                0.0,
            ],
            params: [
                (std::f32::consts::PI / 8.0).tan(),
                aspect,
                near,
                self.distance * 5.0 + 1e7,
            ],
            projection: [
                f32::from(
                    self.projection == beamcraft_engine::model_view::Projection::Orthographic,
                ),
                self.half_span,
                0.,
                0.,
            ],
            clip_plane: self.clip_plane,
            clip_depth: self.clip_depth,
            area_min: self.area_min,
            area_max: self.area_max,
            color: self.color,
        })
    }
}
impl View {
    /// CPU equivalent of shader projection, for camera and pixel-test evidence.
    pub fn project(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let [ex, ey, ez, _] = self.eye;
        let p = [x - ex, y - ey, z - ez];
        let [rx, ry, rz, _] = self.right;
        let [ux, uy, uz, _] = self.up;
        let [bx, by, bz, _] = self.back;
        let [tan, aspect, near, far] = self.params;
        let depth = -dot(p, [bx, by, bz]);
        let [orthographic, span, _, _] = self.projection;
        let divisor = if orthographic > 0.5 {
            span
        } else {
            depth * tan
        };
        let visible = self.contains([x, y, z]);
        [
            dot(p, [rx, ry, rz]) / (divisor * aspect),
            dot(p, [ux, uy, uz]) / divisor,
            if !visible {
                -1.
            } else if orthographic > 0.5 {
                (depth - near) / (far - near)
            } else {
                far / (far - near) - far * near / ((far - near) * depth)
            },
        ]
    }
    /// Whether a scene-relative point is inside this view's depth slab.
    pub fn contains(&self, point: [f32; 3]) -> bool {
        let [a, b, c, on] = self.area_min;
        let [d, e, f, _] = self.area_max;
        let [x, y, z] = point;
        if on > 0.5 && (x < a || y < b || z < c || x > d || y > e || z > f) {
            return false;
        }
        let [above, below, enabled, _] = self.clip_depth;
        let [nx, ny, nz, offset] = self.clip_plane;
        let d = dot(point, [nx, ny, nz]) - offset;
        enabled < 0.5 || (d >= -below && d <= above)
    }
    pub(crate) fn bytes(self) -> Vec<u8> {
        self.right
            .into_iter()
            .chain(self.up)
            .chain(self.back)
            .chain(self.eye)
            .chain(self.params)
            .chain(self.projection)
            .chain(self.clip_plane)
            .chain(self.clip_depth)
            .chain(self.area_min)
            .chain(self.area_max)
            .chain(self.color)
            .flat_map(f32::to_le_bytes)
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_fitted_scene_projects_inside_the_view() {
        // A 6 m x 3 m x 0.5 m box centred at the origin, in millimetres, in landscape and portrait.
        let half = [3000.0f32, 1500.0, 250.0];
        let extent = 2.0 * (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt();
        for aspect in [16.0 / 9.0, 1.0, 0.4] {
            let mut c = Camera::new([0.0; 3], 10.0, aspect).unwrap();
            c.navigate([0.7, -0.2], [0.0; 2], 0.0).unwrap();
            c.fit([0.0; 3], extent, aspect).unwrap();
            let v = c.view(aspect).unwrap();
            for sx in [-1.0f32, 1.0] {
                for sy in [-1.0f32, 1.0] {
                    for sz in [-1.0f32, 1.0] {
                        let [x, y, z] = v.project([sx * half[0], sy * half[1], sz * half[2]]);
                        assert!(
                            x.abs() <= 1.0 && y.abs() <= 1.0 && (0.0..1.0).contains(&z),
                            "aspect {aspect}: {x} {y} {z}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn projection_and_basis() {
        let c = Camera::new([0.0; 3], 100.0, 2.0).unwrap();
        let v = c.view(2.0).unwrap();
        let [x, y, z] = v.project([0.0; 3]);
        assert!(x.abs() < 1e-5 && y.abs() < 1e-5 && (0.0..1.0).contains(&z));
        let [rx, ry, rz, _] = v.right;
        let [ux, uy, uz, _] = v.up;
        let [bx, by, bz, _] = v.back;
        assert!(dot([rx, ry, rz], [ux, uy, uz]).abs() < 1e-6);
        assert!(dot([rx, ry, rz], [bx, by, bz]).abs() < 1e-6);
        for a in [[rx, ry, rz], [ux, uy, uz], [bx, by, bz]] {
            assert!((dot(a, a) - 1.0).abs() < 1e-6);
        }
        let [px, _, _] = v.project([rx * 10.0, ry * 10.0, rz * 10.0]);
        assert!(px > 0.0);
    }
    #[test]
    fn input_bounds_and_navigation() {
        assert!(Camera::new([f32::NAN; 3], 1.0, 1.0).is_err());
        assert!(Camera::new([0.0; 3], 100.0, 0.0).is_err());
        let mut c = Camera::new([0.0; 3], 100.0, 1.0).unwrap();
        assert!(c.navigate([f32::INFINITY, 0.0], [0.0; 2], 0.0).is_err());
        assert!(c.view(0.0).is_err());
        for _ in 0..100 {
            c.navigate([100.0, 100.0], [1e6, -1e6], 100.0).unwrap();
        }
        assert_eq!(c.view(1.0).unwrap().bytes().len(), 176);
        assert!(c.distance().is_finite());
    }
    #[test]
    fn zoom_toward_the_cursor_keeps_the_point_under_the_cursor_still() {
        let aspect = 1.6;
        // A fresh camera per case: the invariant must not depend on navigation history.
        for ndc in [[0.5f32, 0.3], [-0.7, -0.4], [0.9, -0.8], [0.0, 0.0]] {
            for zoom in [0.7f32, -0.9, 2.0] {
                let mut c = Camera::new([0.0; 3], 1000.0, aspect).unwrap();
                let v = c.view(aspect).unwrap();
                let [tan, va, _, _] = v.params;
                let d = c.distance();
                // The anchor is where the cursor ray crosses the plane through the target.
                let anchor = [
                    v.eye[0] - v.back[0] * d
                        + v.right[0] * ndc[0] * tan * va * d
                        + v.up[0] * ndc[1] * tan * d,
                    v.eye[1] - v.back[1] * d
                        + v.right[1] * ndc[0] * tan * va * d
                        + v.up[1] * ndc[1] * tan * d,
                    v.eye[2] - v.back[2] * d
                        + v.right[2] * ndc[0] * tan * va * d
                        + v.up[2] * ndc[1] * tan * d,
                ];
                let want = v.project(anchor);
                let pan = c.cursor_zoom_pan(ndc, aspect, zoom).unwrap();
                c.navigate([0.0; 2], pan, zoom).unwrap();
                let got = c.view(aspect).unwrap().project(anchor);
                assert!(
                    (want[0] - got[0]).abs() < 1e-3 && (want[1] - got[1]).abs() < 1e-3,
                    "ndc {ndc:?} zoom {zoom}: {want:?} -> {got:?}"
                );
            }
        }
        // At the zoom bounds navigate clamps the distance; the compensation follows the clamped
        // distance, so the anchor still stays still (a raw exponential here would miss).
        let mut c = Camera::new([0.0; 3], 1000.0, aspect).unwrap();
        let zoom = 6.0f32;
        let v = c.view(aspect).unwrap();
        let [tan, va, _, _] = v.params;
        let d = c.distance();
        let anchor = [
            v.eye[0] - v.back[0] * d + v.right[0] * 0.5 * tan * va * d + v.up[0] * 0.3 * tan * d,
            v.eye[1] - v.back[1] * d + v.right[1] * 0.5 * tan * va * d + v.up[1] * 0.3 * tan * d,
            v.eye[2] - v.back[2] * d + v.right[2] * 0.5 * tan * va * d + v.up[2] * 0.3 * tan * d,
        ];
        let want = v.project(anchor);
        let pan = c.cursor_zoom_pan([0.5, 0.3], aspect, zoom).unwrap();
        c.navigate([0.0; 2], pan, zoom).unwrap();
        assert_eq!(c.distance(), 10.0); // the raw exponential would have gone to 3.4
        let got = c.view(aspect).unwrap().project(anchor);
        assert!(
            (want[0] - got[0]).abs() < 1e-3 && (want[1] - got[1]).abs() < 1e-3,
            "clamped: {want:?} -> {got:?}"
        );
        // The compensation is clamped into what navigate accepts: at the distance bounds it
        // would otherwise exceed the 1e6 input limit and get the whole frame rejected.
        let mut big = Camera::new([0.0; 3], 1e7, aspect).unwrap();
        let pan = big.cursor_zoom_pan([1.0, 1.0], aspect, 0.4).unwrap();
        assert!(pan[0].abs() <= 1e6 && pan[1].abs() <= 1e6, "{pan:?}");
        assert!(big.navigate([0.0; 2], pan, 0.4).is_ok());
        assert!(c.cursor_zoom_pan([f32::NAN, 0.0], aspect, 1.0).is_err());
        assert!(c.cursor_zoom_pan([2.0, 0.0], aspect, 1.0).is_err());
        assert!(c.cursor_zoom_pan([0.0, 0.0], aspect, f32::NAN).is_err());
    }
    #[test]
    fn the_near_plane_follows_the_distance_and_stays_inside_the_depth_range() {
        for extent in [10.0f32, 1000.0, 1e6] {
            let c = Camera::new([0.0; 3], extent, 1.6).unwrap();
            let v = c.view(1.6).unwrap();
            let near = v.params[2];
            let far = v.params[3];
            assert!((near - c.distance() * 0.01).abs() < near * 1e-4);
            assert!(near > 0.0 && near < far);
        }
        // Every zoom extreme keeps a valid depth range.
        let mut c = Camera::new([0.0; 3], 1000.0, 1.6).unwrap();
        c.navigate([0.0, 0.0], [0.0; 2], -10.0).unwrap();
        c.navigate([0.0, 0.0], [0.0; 2], 10.0).unwrap();
        let v = c.view(1.6).unwrap();
        assert!(v.params[2] > 0.0 && v.params[2] < v.params[3]);
    }
}

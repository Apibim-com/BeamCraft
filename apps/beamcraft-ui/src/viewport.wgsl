// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Original display-only perspective and Lambert lighting; no engineering computation.
struct Camera { right: vec4f, up: vec4f, back: vec4f, eye: vec4f, params: vec4f, projection: vec4f, clip_plane: vec4f, clip_depth: vec4f, area_min: vec4f, area_max: vec4f, color: vec4f }
fn display_color(base: vec3f) -> vec3f { return select(base,camera.color.xyz,camera.color.x >= 0.0); }
@group(0) @binding(0) var<uniform> camera: Camera;
struct Out { @builtin(position) position: vec4f, @location(0) normal: vec3f, @location(1) world: vec3f }
fn clip_view(p: vec3f) {
    if camera.color.w <= 0.0 { discard; }
    if camera.area_min.w > 0.5 && (any(p < camera.area_min.xyz) || any(p > camera.area_max.xyz)) { discard; }
    let d = dot(p, camera.clip_plane.xyz) - camera.clip_plane.w;
    if camera.clip_depth.z > 0.5 && (d > camera.clip_depth.x || d < -camera.clip_depth.y) { discard; }
}
fn rotate(v: vec3f, q: vec4f) -> vec3f {
    return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
}
@vertex fn instanced(
    @location(0) p: vec3f,
    @location(1) n: vec3f,
    @location(2) t: vec3f,
    @location(3) q: vec4f,
) -> Out {
    let v = rotate(p, q) + t - camera.eye.xyz;
    let d = -dot(v, camera.back.xyz);
    let near = camera.params.z;
    let far = camera.params.w;
    var o: Out;
    o.position = vec4f(
        dot(v, camera.right.xyz) / (camera.params.x * camera.params.y),
        dot(v, camera.up.xyz) / camera.params.x,
        far / (far - near) * d - far * near / (far - near),
        d,
    );
    if camera.projection.x > 0.5 {
        o.position = vec4f(dot(v, camera.right.xyz)/(camera.projection.y*camera.params.y),
            dot(v, camera.up.xyz)/camera.projection.y, (d-near)/(far-near), 1.0);
    }
    o.normal = rotate(n, q);
    o.world = rotate(p, q) + t;
    return o;
}
// Cool steel faces keep the engineering model distinct from the application chrome.
@fragment fn shade(o: Out) -> @location(0) vec4f {
    clip_view(o.world);
    let n = normalize(o.normal);
    let base = mix(vec3f(0.38, 0.51, 0.59), vec3f(0.74, 0.80, 0.84), clamp(n.z, 0.0, 1.0));
    let light = 0.55 + 0.45 * max(dot(n, normalize(vec3f(0.3, -0.4, 0.85))), 0.0);
    return vec4f(display_color(base) * light, camera.color.w);
}
// The ghost preview of the member being placed: the same lighting on a distinct amber, so the
// not-yet-committed shape never reads as a model element.
@fragment fn shade_ghost(o: Out) -> @location(0) vec4f {
    clip_view(o.world);
    let n = normalize(o.normal);
    let light = 0.55 + 0.45 * max(dot(n, normalize(vec3f(0.3, -0.4, 0.85))), 0.0);
    return vec4f(vec3f(0.85, 0.55, 0.1) * light, 1.0);
}
// The grey mode: the same lighting on one neutral grey, without the part colours.
@fragment fn shade_grey(o: Out) -> @location(0) vec4f {
    clip_view(o.world);
    let n = normalize(o.normal);
    let light = 0.55 + 0.45 * max(dot(n, normalize(vec3f(0.3, -0.4, 0.85))), 0.0);
    return vec4f(display_color(vec3f(0.62, 0.62, 0.62)) * light, camera.color.w);
}
// The transparent mode: the shaded colours at partial opacity; the pipeline blends and keeps
// depth testing but not depth writing, so hidden faces show through.
@fragment fn shade_transparent(o: Out) -> @location(0) vec4f {
    clip_view(o.world);
    let n = normalize(o.normal);
    let base = mix(vec3f(0.47, 0.71, 0.77), vec3f(0.96, 0.94, 0.88), clamp(n.z, 0.0, 1.0));
    let light = 0.55 + 0.45 * max(dot(n, normalize(vec3f(0.3, -0.4, 0.85))), 0.0);
    return vec4f(display_color(base) * light, 0.45 * camera.color.w);
}

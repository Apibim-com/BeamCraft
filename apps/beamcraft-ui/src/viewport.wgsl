// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim
// Original display-only perspective and Lambert lighting; no engineering computation.
struct Camera { right: vec4f, up: vec4f, back: vec4f, eye: vec4f, params: vec4f }
@group(0) @binding(0) var<uniform> camera: Camera;
struct Out { @builtin(position) position: vec4f, @location(0) normal: vec3f }
@vertex fn instanced(@location(0) p: vec3f, @location(1) n: vec3f, @location(2) t: vec3f) -> Out {
    let v = p + t - camera.eye.xyz;
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
    o.normal = n;
    return o;
}
@fragment fn shade(o: Out) -> @location(0) vec4f {
    let light = 0.25 + 0.75 * max(dot(normalize(o.normal), normalize(vec3f(0.3, -0.4, 0.85))), 0.0);
    return vec4f(vec3f(0.18, 0.55, 0.82) * light, 1.0);
}

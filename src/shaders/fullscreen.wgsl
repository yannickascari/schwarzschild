struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );

    let pos = positions[idx];
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = (pos + vec2<f32>(1.0)) * 0.5;
    return out;
}

fn ray_sphere(ro: vec3<f32>, rd: vec3<f32>, r: f32) -> f32 {
    let a = dot(rd, rd);
    let b = 2.0 * dot(ro, rd);
    let c = dot(ro, ro) - r * r;

    let d = b * b - 4.0 * a * c;

    if d < 0.0 {
        return -1.0; // No intersection
    }

    return (-b - sqrt(d)) / (2.0 * a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let aspect = 16.0 / 9.0;
    var uv = (in.uv * 2.0 - vec2<f32>(1.0)) * vec2<f32>(aspect, 1.0);

    let ro = vec3<f32>(0.0, 0.0, 3.0);
    let rd = normalize(vec3<f32>(uv, -1.0));

    let t = ray_sphere(ro, rd, 1.0);

    if t > 0.0 {
        let hit = ro + t * rd;

        let normal = normalize(hit);
        return vec4<f32>(normal * 0.5 + vec3<f32>(0.5), 1.0);
    }

    let sky = mix(
        vec3<f32>(0.1, 0.1, 0.2),
        vec3<f32>(0.0, 0.0, 0.0),
        in.uv.y
    );
 
    return vec4<f32>(sky, 1.0);
}
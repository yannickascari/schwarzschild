@group(0) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;

const RS: f32 = 1.0;
const R_MAX: f32 = 30.0;
const DPHI: f32 = 0.03;
const MAX_STEPS: u32 = 500u;

fn sky_color(dir: vec3<f32>) -> vec3<f32> {
    let phi = atan2(dir.z, dir.x);
    let theta = acos(clamp(dir.y, -1.0, 1.0));

    let grid = f32(
        abs(sin(phi * 9.0)) < 0.15 ||
        abs(sin(theta * 9.0)) < 0.15
    );

    let sky = mix(
        vec3<f32>(0.8, 0.3, 0.05),
        vec3<f32>(0.05, 0.1, 0.5),
        clamp(dir.y * 0.5 + 0.5, 0.0, 1.0)
    );

    return mix(sky, vec3<f32>(1.0), grid * 0.9);
}

fn trace_ray(ro: vec3<f32>, rd: vec3<f32>) -> vec4<f32> {
    let r0 = length(ro);

    let b_vec = cross(ro, rd);
    let b = length(b_vec);

    if b < 0.0001 { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }

    let n = normalize(b_vec);
    let e_r = normalize(ro);
    let e_phi = normalize(cross(n, e_r));

    var u = 1.0 / r0;
    var du = -dot(e_r, rd) / b;
    var phi = 0.0;

    for (var i = 0u; i < MAX_STEPS; i++) {
        let r = 1.0 / max(u, 0.0001);

        if r <= RS { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }

        if r >= R_MAX {
            let exit_dir = normalize(
                (-du / (u * u)) * (cos(phi) * e_r + sin(phi) * e_phi) +
                (1.0 / u) * (-sin(phi) * e_r + cos(phi) * e_phi)
            );
            return vec4<f32>(sky_color(exit_dir), 1.0);
        }

        let k1u = du;
        let k1v = -u + 1.5 * RS * u * u;

        let u2 = u + 0.5 * DPHI * k1u;
        let k2u = du + 0.5 * DPHI * k1v;
        let k2v = -u2 + 1.5 * RS * u2 * u2;

        let u3 = u + 0.5 * DPHI * k2u;
        let k3u = du + 0.5 * DPHI * k2v;
        let k3v = -u3 + 1.5 * RS * u3 * u3;

        let u4 = u + DPHI * k3u;
        let k4u = du + DPHI * k3v;
        let k4v = -u4 + 1.5 * RS * u4 * u4;

        u += DPHI / 6.0 * (k1u + 2.0 * k2u + 2.0 * k3u + k4u);
        du += DPHI / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v);
        phi += DPHI;
    }

    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(output_texture);
    if gid.x >= dims.x || gid.y >= dims.y { return; }

    let uv = vec2<f32>(
        f32(gid.x) / f32(dims.x),
        1.0 - f32(gid.y) / f32(dims.y)
    );

    let aspect = f32(dims.x) / f32(dims.y);
    let uv_c = (uv * 2.0 - vec2<f32>(1.0)) * vec2<f32>(aspect, 1.0);

    let ro = vec3<f32>(0.0, 0.0, 15.0);
    let rd = normalize(vec3<f32>(uv_c * 0.5, -1.0));

    textureStore(output_texture, vec2<i32>(gid.xy), trace_ray(ro, rd));
}
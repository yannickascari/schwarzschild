@group(0) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;

struct CameraUniform {
    ro: vec3<f32>,
    fov: f32,
    forward: vec3<f32>,
    _pad1: f32,
    right: vec3<f32>,
    _pad2: f32,
    up: vec3<f32>,
    _pad3: f32
}

@group(0) @binding(1) var<uniform> camera: CameraUniform;

const RS: f32        = 1.0;
const R_MAX: f32     = 30.0;
const DPHI: f32      = 0.03;
const MAX_STEPS: u32 = 500u;
const R_INNER: f32   = 3.0;
const R_OUTER: f32 = 12.0;

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

fn disk_color(pos3d: vec3<f32>, photon_dir: vec3<f32>, r: f32) -> vec4<f32> {
    let t = 1.0 - clamp((r - R_INNER) / (R_OUTER - R_INNER), 0.0, 1.0);
    let hot = vec3<f32>(1.0, 0.95, 0.8);
    let warm = vec3<f32>(1.0, 0.5, 0.1);
    let cool = vec3<f32>(0.5, 0.08, 0.02);

    var base: vec3<f32>;
    if t > 0.5 {
        base = mix(warm, hot, (t - 0.5) * 2.0);
    } else {
        base = mix(cool, warm, t * 2.0);
    }

    let v_orb = sqrt(RS / (2.0 * r));

    let orbit_dir = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), normalize(pos3d)));

    let cos_theta = dot(orbit_dir, normalize(photon_dir));
    let doppler = 1.0 / max(1.0 - v_orb * cos_theta, 0.05);

    let grav = sqrt(max(1.0 - RS / r, 0.0));

    let intensity = pow(doppler, 3.0) * grav;

    return vec4<f32>(base * intensity, 1.0);
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

    var prev_y = ro.y;

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

        let pos3d = (1.0 / u) * (cos(phi) * e_r + sin(phi) * e_phi);
        let curr_y = pos3d.y;

        if prev_y * curr_y < 0.0 && r > R_INNER && r < R_OUTER {
            let photon_dir = normalize(
                (-du / (u * u)) * (cos(phi) * e_r + sin(phi) * e_phi) +
                (1.0 / u) * (-sin(phi) * e_r + cos(phi) * e_phi)
            );
            return disk_color(pos3d, photon_dir, r);
        }

        prev_y = curr_y;

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
        1.0 - f32(gid.y) / f32(dims.y),
    );

    let aspect = f32(dims.x) / f32(dims.y);
    let uv_c = (uv * 2.0 - vec2<f32>(1.0)) * vec2<f32>(aspect, 1.0);

    let rd = normalize(
        uv_c.x * camera.right +
        uv_c.y * camera.up +
        camera.forward / camera.fov
    );

    textureStore(output_texture, vec2<i32>(gid.xy), trace_ray(camera.ro, rd));
}
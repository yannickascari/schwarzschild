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

const RS: f32         = 1.0;
const PI: f32         = 3.14159265;
const CELL_RATIO: f32 = 0.9;
const R_MAX: f32      = 60.0;
const DPHI: f32       = 0.01;
const MAX_STEPS: u32  = 500u;
const R_INNER: f32    = 3.0;
const R_OUTER: f32    = 12.0;

fn hash2d(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn sky_color(dir: vec3<f32>) -> vec3<f32> {
    let u = atan2(dir.z, dir.x) / (2.0 * PI) + 0.5;
    let v = acos(clamp(dir.y, -1.0, 1.0)) / PI;
    let uv = vec2<f32>(u, v);

    let N = 300.0;
    let cell = floor(uv * N);
    let cell_uv = fract(uv * N);

    let star_x = hash2d(cell);
    let star_y = hash2d(cell + vec2<f32>(13.7, 27.3));
    let star_b = hash2d(cell + vec2<f32>(57.1, 113.5));

    var star = 0.0;
    if star_b > CELL_RATIO {
        let d = length(cell_uv - vec2<f32>(star_x, star_y));
        let size = 0.04 + (star_b - CELL_RATIO) * 1.5;
        star = smoothstep(size, 0.0, d) * (star_b - CELL_RATIO) * 25.0;
    }

    let star_color = mix(
        vec3<f32>(0.8, 0.9, 1.0),
        vec3<f32>(1.0, 0.9, 0.7),
        hash2d(cell + vec2<f32>(3.3, 7.7))
    );

    let milky = exp(-abs(dir.y) * 5.0) * 0.08;

    return vec3<f32>(0.0, 0.0, 0.015)
        + vec3<f32>(0.3, 0.35, 0.5) * milky
        + star_color * star;
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

    let intensity = pow(doppler, 1.5) * grav;

    let outer_fade = smoothstep(R_OUTER, R_OUTER - 2.0, r);
    let inner_fade = smoothstep(R_INNER, R_INNER + 1.5, r);
    let alpha = outer_fade * inner_fade;

    return vec4<f32>(base * intensity * alpha, 1.0);
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

    var disk_rgb = vec3<f32>(0.0);
    var disk_opacity = 0.0;

    for (var i = 0u; i < MAX_STEPS; i++) {
        let r = 1.0 / max(u, 0.0001);

        if r <= RS { return vec4<f32>(disk_rgb, 1.0); }

        if r >= R_MAX {
            let exit_dir = normalize(
                (-du / (u * u)) * (cos(phi) * e_r + sin(phi) * e_phi) +
                (1.0 / u) * (-sin(phi) * e_r + cos(phi) * e_phi)
            );
            let sky = sky_color(exit_dir);
            return vec4<f32>(disk_rgb + sky * (1.0 - disk_opacity), 1.0);
        }

        let pos3d = (1.0 / u) * (cos(phi) * e_r + sin(phi) * e_phi);

        if r > R_INNER && r < R_OUTER && disk_opacity < 0.99 {
            let H = r * 0.03;
            let weight = exp(-pos3d.y * pos3d.y / (2.0 * H * H)) * DPHI * 10.0;

            if weight > 0.001 {
                let photon_dir = normalize(
                    (-du / (u * u)) * (cos(phi) * e_r + sin(phi) * e_phi) +
                    (1.0 / u) * (-sin(phi) * e_r + cos(phi) * e_phi)
                );

                let dc = disk_color(pos3d, photon_dir, r);
                let contrib = weight * dc.a;
                disk_rgb += dc.xyz * contrib * (1.0 - disk_opacity);
                disk_opacity = min(disk_opacity + contrib * (1.0 - disk_opacity), 1.0);
            }
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

    return vec4<f32>(disk_rgb, 1.0);
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
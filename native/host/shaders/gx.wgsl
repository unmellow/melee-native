struct Uniforms {
    mvp: mat4x4<f32>,
    tev_color: vec4<f32>,
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var gx_tex: texture_2d<f32>;
@group(0) @binding(2) var gx_samp: sampler;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) nrm: vec3<f32>,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var o: VsOut;
    o.clip = u.mvp * vec4<f32>(v.pos, 1.0);
    o.color = v.color;
    o.uv = v.uv;
    o.nrm = v.nrm;
    return o;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    var texc = vec4<f32>(1.0);
    if (u.params.y > 0.5) {
        texc = textureSample(gx_tex, gx_samp, v.uv);
    }
    let mode = u32(u.params.x + 0.5);
    var outc: vec4<f32>;
    switch mode {
        case 3u: { outc = texc; }
        case 1u: { outc = vec4<f32>(mix(v.color.rgb, texc.rgb, texc.a), v.color.a); }
        case 2u: { outc = vec4<f32>(mix(v.color.rgb, texc.rgb, u.tev_color.a), v.color.a); }
        case 4u: { outc = v.color; }
        default: { outc = v.color * texc; }
    }
    let light = 0.55 + 0.45 * max(dot(normalize(v.nrm), vec3<f32>(0.25, 0.8, 0.4)), 0.0);
    if (u.params.y < 0.5 && mode != 4u) {
        outc = vec4<f32>(outc.rgb * light, outc.a);
    }
    if (outc.a < 0.02) {
        discard;
    }
    return outc;
}

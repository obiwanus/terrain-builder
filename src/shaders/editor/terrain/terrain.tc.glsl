#version 450 core

layout(vertices = 4) out;

in VS_OUT { vec2 tile_uv; }
tcs_in[];

out TCS_OUT { vec2 tile_uv; }
tcs_out[];

layout(std140, binding = 1) uniform UTransforms {
    mat4 mvp;
    mat4 proj;
    mat4 view;
    mat4 model;
    mat4 sun_vp;
}
uTransforms;

uniform float tess_level;

void main() {
    if (gl_InvocationID == 0) {
        vec4 p0 = uTransforms.mvp * gl_in[0].gl_Position;
        vec4 p1 = uTransforms.mvp * gl_in[1].gl_Position;
        vec4 p2 = uTransforms.mvp * gl_in[2].gl_Position;
        vec4 p3 = uTransforms.mvp * gl_in[3].gl_Position;

        if (p0.z <= 0.0 && p1.z <= 0.0 && p2.z <= 0.0 && p3.z <= 0.0) {
            // Patch is behind the camera - cull
            // TODO: understand why visible patches are culled sometimes
            gl_TessLevelOuter[0] = 0.0;
            gl_TessLevelOuter[1] = 0.0;
            gl_TessLevelOuter[2] = 0.0;
            gl_TessLevelOuter[3] = 0.0;
        } else {
            // float l0 = length(p2.xy - p0.xy) + 1.0;
            // float l1 = length(p3.xy - p2.xy) + 1.0;
            // float l2 = length(p3.xy - p1.xy) + 1.0;
            // float l3 = length(p1.xy - p0.xy) + 1.0;
            float l0 = tess_level;
            float l1 = tess_level;
            float l2 = tess_level;
            float l3 = tess_level;

            gl_TessLevelOuter[0] = l0;
            gl_TessLevelOuter[1] = l1;
            gl_TessLevelOuter[2] = l2;
            gl_TessLevelOuter[3] = l3;

            gl_TessLevelInner[0] = min(l1, l3);
            gl_TessLevelInner[1] = min(l0, l2);
        }
    }

    gl_out[gl_InvocationID].gl_Position = gl_in[gl_InvocationID].gl_Position;
    tcs_out[gl_InvocationID].tile_uv = tcs_in[gl_InvocationID].tile_uv;
}

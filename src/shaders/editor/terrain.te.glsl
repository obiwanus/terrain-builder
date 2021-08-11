#version 450 core

layout(quads, fractional_odd_spacing) in;

uniform sampler2D heightmap;
uniform mat4 mvp;

const float MAX_HEIGHT = 200.0;

in TCS_OUT { vec2 tile_uv; }
tes_in[];

out TES_OUT {
    vec2 tile_uv;
    vec3 frag_pos;
}
tes_out;

void main() {
    vec2 uv1 = mix(tes_in[0].tile_uv, tes_in[1].tile_uv, gl_TessCoord.x);
    vec2 uv2 = mix(tes_in[2].tile_uv, tes_in[3].tile_uv, gl_TessCoord.x);
    vec2 tile_uv = mix(uv2, uv1, gl_TessCoord.y);

    vec4 p1 = mix(gl_in[0].gl_Position, gl_in[1].gl_Position, gl_TessCoord.x);
    vec4 p2 = mix(gl_in[2].gl_Position, gl_in[3].gl_Position, gl_TessCoord.x);
    vec4 p = mix(p2, p1, gl_TessCoord.y);

    // TODO: add displacement
    p.y += texture(heightmap, tile_uv).r * MAX_HEIGHT;
    gl_Position = mvp * p;
    tes_out.tile_uv = tile_uv;
    tes_out.frag_pos = p.xyz;
}

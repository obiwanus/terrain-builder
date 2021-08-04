#version 410 core

// in VS_OUTPUT {
//     vec3 normal;
//     vec3 frag_pos;
//     vec3 frag_pos_view;
//     vec3 color;
//     vec2 tex_coord;
// }
// IN;

// struct Material {
//     vec3 specular;
//     float shininess;
// };

// struct DirectionalLight {
//     vec3 direction;

//     vec3 ambient;
//     vec3 diffuse;
//     vec3 specular;
// };

in TES_OUT { vec2 uv; }
fs_in;

out vec4 Color;

// uniform vec3 cursor;
// uniform float brush_size;
uniform sampler2D terrain_texture;

// uniform Material material;
// uniform DirectionalLight directional_light;

// vec3 calc_directional_light(DirectionalLight light, vec3 normal, vec3 view_direction) {
//     vec3 light_direction = normalize(-light.direction);
//     vec3 diffuse_color = IN.color;

//     // Diffuse
//     float diff = max(dot(normal, light_direction), 0.0);

//     // Specular
//     vec3 reflection = reflect(-light_direction, normal);
//     float spec = pow(max(dot(view_direction, reflection), 0.0), material.shininess);

//     // Result
//     vec3 ambient = light.ambient * diffuse_color;
//     vec3 diffuse = light.diffuse * diff * diffuse_color;
//     vec3 specular = light.specular * spec * material.specular;
//     return (ambient + diffuse + specular);
// }

// void main() {
//     vec3 normal = normalize(IN.normal);
//     vec3 view_direction = normalize(-IN.frag_pos_view);

//     // vec4 base_color = vec4(calc_directional_light(directional_light, normal,
//     // view_direction), 1.0);

//     vec4 base_color = vec4(0.7, 0.2, 0.7, 1.0);

//     // Cursor
//     float distance_to_cursor = clamp(distance(IN.frag_pos.xz, cursor.xz) / brush_size, 0.2, 1.0);
//     if (0.99 < distance_to_cursor && distance_to_cursor < 1.0) {
//         distance_to_cursor = 0.7;  // to create a border
//     }

//     base_color =
//         mix(base_color, vec4(texture(terrain_texture, IN.tex_coord).xyz, 1.0),
//         distance_to_cursor);

//     if (distance_to_cursor < 1.0) {
//         Color = base_color * clamp(0.1 * IN.frag_pos.y, 0.7, 1.0);
//     } else {
//         Color = base_color * clamp(0.1 * IN.frag_pos.y, 0.2, 1.0);
//     }
// }

void main() { Color = texture(terrain_texture, fs_in.uv); }
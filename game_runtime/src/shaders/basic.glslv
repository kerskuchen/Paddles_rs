#version 150 core

in vec4 a_pos;
in vec3 a_uv;
in vec4 a_color_modulate;
in float a_additivity;

out vec4 v_color_modulate;
out float v_additivity;
out vec3 v_uv;

uniform mat4 u_transform;
uniform int u_use_texture_array;

void main() {
    v_color_modulate = a_color_modulate;
    v_additivity = a_additivity;
    v_uv = a_uv;

    gl_Position = u_transform * a_pos;
}

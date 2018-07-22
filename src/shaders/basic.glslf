#version 150 core

uniform sampler2D u_Sampler;

in vec4 v_Color;
in vec2 v_Uv;

out vec4 Target0;

void main() {
    vec4 tex_color = texture(u_Sampler, v_Uv);
    float tex_alpha = tex_color.a;
    vec3 rgb_color = tex_color.rgb * tex_alpha + v_Color.rgb * (1.0 - tex_alpha);
    Target0 = vec4(rgb_color, 1.0);
}

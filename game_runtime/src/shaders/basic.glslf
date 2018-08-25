#version 150 core

uniform sampler2D u_Sampler;
uniform sampler2DArray u_SamplerArray;

in vec4 v_Color;
in vec3 v_Uv;

out vec4 Target0;

uniform int u_UseTextureArray;

void main() {
    vec4 tex_color = vec4(0.0, 0.0, 0.0, 0.0);

    if (u_UseTextureArray == 1) {
        tex_color = texture(u_SamplerArray, v_Uv);
    } else {
        tex_color = texture(u_Sampler, vec2(v_Uv.x, v_Uv.y));
    }
    
    // Transparent parts of a texture will be filled with with the vertex color
    float tex_alpha = tex_color.a;
    vec3 rgb_color = tex_color.rgb * tex_alpha + v_Color.rgb * (1.0 - tex_alpha);
    Target0 = vec4(rgb_color, 1.0);
}

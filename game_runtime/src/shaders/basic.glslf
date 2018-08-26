#version 150 core

uniform sampler2D u_Sampler;
uniform sampler2DArray u_SamplerArray;

in vec4 v_Color;
in vec3 v_Uv;

out vec4 Target0;

uniform int u_UseTextureArray;

void main() {
    vec4 tex_color; 
    if (u_UseTextureArray == 1) {
        tex_color = texture(u_SamplerArray, v_Uv);
    } else {
        tex_color = texture(u_Sampler, vec2(v_Uv.x, v_Uv.y));
    }

    vec4 tex_premultiplied = vec4(tex_color.r * tex_color.a,
                                  tex_color.g * tex_color.a,
                                  tex_color.b * tex_color.a,
                                  tex_color.a);
    
    vec4 color = tex_premultiplied * v_Color;

    if (dot(color, color) == 0.0) {
        discard;
    }
    Target0 = color;
}

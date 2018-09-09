#version 150 core

uniform sampler2D u_sampler;
uniform sampler2DArray u_sampler_array;

in vec4 v_color_modulate;
in float v_additivity;
in vec3 v_uv;

out vec4 out_color_0;

uniform int u_use_texture_array;

void main() {
    vec4 tex_color; 
    if (u_use_texture_array == 1) {
        tex_color = texture(u_sampler_array, v_uv);
    } else {
        tex_color = texture(u_sampler, vec2(v_uv.x, v_uv.y));
    }

    // This effectively does the following:
    // vec3 tex_color_premultiplied = tex_color.rgb * tex_color.a;
    // vec3 color_modulate_premultiplied = v_color_modulate.rgb * v_color_modulate.a;
    // vec3 color_premultiplied = tex_color_premultiplied * color_modulate_premultiplied;
    // vec4 color = vec4(color_premultiplied, tex_color.a * v_color_modulate.a * (1.0 - additivity));
    //
    vec4 color = vec4((tex_color.r * v_color_modulate.r) * (tex_color.a * v_color_modulate.a),
                      (tex_color.g * v_color_modulate.g) * (tex_color.a * v_color_modulate.a),
                      (tex_color.b * v_color_modulate.b) * (tex_color.a * v_color_modulate.a),
                      (tex_color.a * v_color_modulate.a) * (1.0 - v_additivity));

    if (dot(color, color) == 0.0) {
        discard;
    }
    out_color_0 = color;
}

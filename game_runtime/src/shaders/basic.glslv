#version 150 core

in vec4 a_Pos;
in vec3 a_Uv;
in vec4 a_Color;

out vec4 v_Color;
out vec3 v_Uv;

uniform mat4 u_Transform;
uniform int u_UseTextureArray;

void main() {
    v_Color = a_Color;
    v_Uv = a_Uv;
    gl_Position = u_Transform * a_Pos;
}

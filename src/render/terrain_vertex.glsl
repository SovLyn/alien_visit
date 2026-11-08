#version 300 es
precision highp float;
layout(location=0) in vec3 a_position;
layout(location=1) in vec3 a_normal;
uniform mat4 u_vp;
uniform mat4 u_light_vp;
uniform vec3 u_offset;
out vec3 v_pos;
out vec3 v_normal;
out vec3 v_mesh;
out vec4 v_shadow;
void main() {
    v_mesh = a_position;
    v_pos = a_position + u_offset;
    v_normal = a_normal;
    v_shadow = u_light_vp * vec4(v_pos, 1.0);
    gl_Position = u_vp * vec4(v_pos, 1.0);
}

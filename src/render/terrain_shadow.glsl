#version 300 es
precision highp float;
layout(location=0) in vec3 a_position;
uniform mat4 u_light_vp;
uniform vec3 u_offset;
void main() { gl_Position = u_light_vp * vec4(a_position + u_offset, 1.0); }

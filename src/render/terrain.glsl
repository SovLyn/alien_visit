#version 300 es
precision highp float;
in vec3 v_pos;
in vec3 v_normal;
in vec3 v_mesh;
in vec4 v_shadow;
uniform sampler2D u_shadow;
uniform float u_shadow_texel;
uniform vec2 u_origin;
uniform vec3 u_sun;
uniform vec3 u_vhal;
uniform float u_exposure;
uniform float u_haze;
uniform float u_vhal_light;
uniform float u_sun_visibility;
out vec4 color;
float hash(vec2 p) { return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453); }
float noise(vec2 p) {
    vec2 i=floor(p), f=fract(p); f=f*f*(3.0-2.0*f);
    return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+1.0),f.x),f.y);
}
// Near-field regolith relief. Frequencies are in metres; keep the finest
// octave above the shadow-map texel so this reads as grain, not acne.
float detail_low(vec2 p) {
    float h=0.0;
    h+=noise(p*0.74+vec2(13.2,4.7))*0.52;
    h+=noise(p*2.21+vec2(7.4,1.3))*0.29;
    h+=sin(dot(p,vec2(0.82,0.57))*1.15+noise(p*0.075)*5.0)*0.035;
    return h;
}
float detail_high(vec2 p) {
    return noise(p*6.40+vec2(2.1,8.8))*0.13;
}
float shadowing(vec3 n) {
    vec3 q=v_shadow.xyz/v_shadow.w*0.5+0.5;
    if(any(lessThan(q,vec3(0.0))) || any(greaterThan(q,vec3(1.0)))) return 1.0;
    float bias=max(0.00008, 0.00030*(1.0-max(dot(n,u_sun),0.0)));
    float s=0.0;
    for(int y=-1;y<=1;y++) for(int x=-1;x<=1;x++) {
        float depth=texture(u_shadow,q.xy+vec2(float(x),float(y))*u_shadow_texel).r;
        s += q.z-bias <= depth ? 1.0 : 0.0;
    }
    return s/9.0;
}
vec3 aces(vec3 x) { return clamp((x*(2.51*x+0.03))/(x*(2.43*x+0.59)+0.14),0.0,1.0); }
vec3 srgb(vec3 x) { return mix(12.92*x,1.055*pow(max(x,vec3(0.0)),vec3(1.0/2.4))-0.055,step(vec3(0.0031308),x)); }
void main() {
    vec3 n=normalize(v_normal);
    vec2 world=vec2(v_mesh.x,-v_mesh.z)+u_origin;
    float dist=length(v_pos.xz);
    float footprint=max(fwidth(world.x),fwidth(world.y));
    // Two normal bands with independent pixel-footprint fades. The low
    // band keeps a little relief through the middle ground; the high band
    // is only resolved close to the camera. This avoids the visible
    // "texture patch" edge of a single near cutoff.
    float low_fade=1.0-smoothstep(0.10,1.05,footprint*2.21);
    float high_fade=1.0-smoothstep(0.08,0.55,footprint*6.40);

    float e=0.16;
    float l0=detail_low(world);
    vec2 grad_low=(vec2(detail_low(world+vec2(e,0.0)),detail_low(world+vec2(0.0,e)))-l0)/e;
    float h0=detail_high(world);
    vec2 grad_high=(vec2(detail_high(world+vec2(e,0.0)),detail_high(world+vec2(0.0,e)))-h0)/e;
    n=normalize(n
        +vec3(-grad_low.x,0.0,-grad_low.y)*0.055*low_fade
        +vec3(-grad_high.x,0.0,-grad_high.y)*0.035*high_fade);

    float mineral=noise(world/730.0)*0.7+noise(world/270.0+17.2)*0.3;
    float slope=clamp((1.0-n.y)*4.0,0.0,1.0);
    vec3 dust=pow(vec3(122.,111.,96.)/255.0,vec3(2.2));
    vec3 regolith=pow(vec3(103.,93.,81.)/255.0,vec3(2.2));
    vec3 pale=pow(vec3(145.,132.,111.)/255.0,vec3(2.2));
    vec3 basalt=pow(vec3(69.,68.,63.)/255.0,vec3(2.2));
    vec3 slate=pow(vec3(85.,92.,92.)/255.0,vec3(2.2));
    vec3 albedo=mix(regolith,dust,smoothstep(0.22,0.8,mineral));
    albedo=mix(albedo,pale,0.3*smoothstep(0.64,0.85,mineral));
    albedo=mix(albedo,mix(basalt,slate,mineral),slope*0.7);
    float strata=sin(v_mesh.y*0.31+noise(world/85.0)*2.0);
    float grain=noise(world*0.45);
    albedo *= 0.91+0.07*grain+0.035*strata;

    // Fine dry regolith colour, kept low-contrast and LOD-faded: broad dust
    // drift, fine grit, and a near-field grain. No stamped pebble cells.
    float mid_a=noise(world*0.35+vec2(4.1,7.7));
    float fine_a=noise(world*1.35+vec2(9.2,3.4));
    float fine_b=noise(world*4.70+vec2(2.8,11.6));
    float fine_c=noise(world*13.0+vec2(7.1,1.2));
    float fade_a=1.0-smoothstep(0.15,0.90,footprint*1.35);
    float fade_b=1.0-smoothstep(0.08,0.40,footprint*4.70);
    float fade_c=1.0-smoothstep(0.06,0.30,footprint*13.0);
    albedo *= 1.0+0.018*(mid_a-0.5)
                 +0.022*(fine_a-0.5)*fade_a
                 +0.016*(fine_b-0.5)*fade_b
                 +0.010*(fine_c-0.5)*fade_c;

    float wind=sin(dot(world,vec2(0.86,0.51))*1.9+noise(world*0.07)*6.0);
    albedo *= 1.0+0.012*wind*(1.0-smoothstep(0.10,0.50,footprint*1.9));
    // Slight warm-neutral mineral correction: the blue ambient otherwise
    // pushes the desert toward olive.
    albedo *= vec3(1.035,1.0,0.955);
    float direct=max(dot(n,u_sun),0.0)*smoothstep(-0.02,0.02,u_sun.y)*u_sun_visibility*shadowing(n);
    float reflected=max(dot(n,u_vhal),0.0)*max(u_vhal.y,0.0);
    vec3 illumination=vec3(0.158,0.182,0.232)
        +vec3(0.115,0.220,0.270)*u_vhal_light*(0.35+0.65*reflected)
        +vec3(5.20,5.85,6.80)*direct;
    vec3 lit=albedo*illumination;
    float fog=smoothstep(6000.0,28000.0,dist)*clamp(u_haze,0.0,1.0);
    // Fade the outermost mesh even when haze is disabled: the sky remains seamless.
    fog=max(fog,smoothstep(26000.0,29900.0,dist));
    vec3 horizon=vec3(0.030,0.044,0.054);
    vec3 mapped=srgb(aces(mix(lit,horizon*0.72,fog)*u_exposure));
    mapped+=(hash(gl_FragCoord.xy)-0.5)/255.0;
    color=vec4(mapped,1.0);
}

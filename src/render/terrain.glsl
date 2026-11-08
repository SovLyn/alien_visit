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
// Value noise and its analytic gradient. The latter avoids both the extra
// samples and the scale-dependent blur of finite-difference bump mapping.
vec3 regolith_noise(vec2 p) {
    vec2 i=floor(p), f=fract(p);
    vec2 u=f*f*(3.0-2.0*f), du=6.0*f*(1.0-f);
    float a=hash(i), b=hash(i+vec2(1,0));
    float c=hash(i+vec2(0,1)), d=hash(i+1.0);
    float value=mix(mix(a,b,u.x),mix(c,d,u.x),u.y);
    vec2 gradient=vec2(mix(b-a,d-c,u.y)*du.x,
                       mix(c-a,d-b,u.x)*du.y);
    return vec3(value,gradient);
}
// Fade each octave independently as its projected cell approaches a pixel.
// The zero-mean detail leaves the same average surface at every distance.
float detail_visibility(float footprint, float frequency) {
    return 1.0-smoothstep(0.10,0.50,footprint*frequency);
}
float shadowing(vec3 n) {
    vec3 q=v_shadow.xyz/v_shadow.w*0.5+0.5;
    if(any(lessThan(q,vec3(0.0))) || any(greaterThan(q,vec3(1.0)))) return 1.0;
    float bias=max(0.00008, 0.00030*(1.0-max(dot(n,u_sun),0.0)));
    // Steep faces are especially prone to self-shadow bands at grazing sun.
    // Leave the nearly level receiving ground (and its long shadow) alone.
    float steep=1.0-smoothstep(0.35,0.85,n.y);
    bias+=0.00025*steep;
    float s=0.0;
    for(int y=-1;y<=1;y++) for(int x=-1;x<=1;x++) {
        float depth=texture(u_shadow,q.xy+vec2(float(x),float(y))*u_shadow_texel).r;
        s += q.z-bias <= depth ? 1.0 : 0.0;
    }
    if(steep>0.0) {
        // Widen PCF only on steep rock, reducing self-shadow stair steps
        // without softening the ground's long cast shadow.
        float wide=s;
        for(int y=-2;y<=2;y++) for(int x=-2;x<=2;x++) {
            if(abs(x)==2 || abs(y)==2) {
                float depth=texture(u_shadow,q.xy+vec2(float(x),float(y))*u_shadow_texel).r;
                wide += q.z-bias <= depth ? 1.0 : 0.0;
            }
        }
        return mix(s/9.0,wide/25.0,steep);
    }
    return s/9.0;
}
vec3 aces(vec3 x) { return clamp((x*(2.51*x+0.03))/(x*(2.43*x+0.59)+0.14),0.0,1.0); }
vec3 srgb(vec3 x) { return mix(12.92*x,1.055*pow(max(x,vec3(0.0)),vec3(1.0/2.4))-0.055,step(vec3(0.0031308),x)); }
void main() {
    vec3 surface_normal=normalize(v_normal);
    vec2 world=vec2(v_mesh.x,-v_mesh.z)+u_origin;
    float dist=length(v_pos.xz);
    float footprint=max(fwidth(world.x),fwidth(world.y));

    // Successive spatial bands bridge broad dust drifts, sub-metre relief,
    // and centimetre grains. Heights are in metres, not colour contrast.
    mat2 rotation=mat2(0.8,0.6,-0.6,0.8);
    mat2 micro_rotation=mat2(0.28,0.96,-0.96,0.28);
    float broad_vis=detail_visibility(footprint,0.028);
    float drift_vis=detail_visibility(footprint,0.27);
    float coarse_vis=detail_visibility(footprint,3.0);
    float medium_vis=detail_visibility(footprint,10.0);
    float fine_vis=detail_visibility(footprint,20.0);
    float micro_vis=detail_visibility(footprint,40.0);
    // Derivatives are evaluated above the branches; fully filtered octaves
    // do no hashing, particularly over the distant majority of the terrain.
    vec3 broad=vec3(0.5,0.0,0.0), drift=broad, coarse=broad;
    vec3 medium=broad, fine=broad, micro=broad;
    if(broad_vis>0.0) broad=regolith_noise(rotation*world*0.028+vec2(19.3,3.1));
    if(drift_vis>0.0) drift=regolith_noise(transpose(rotation)*world*0.27+vec2(8.2,9.7));
    if(coarse_vis>0.0) coarse=regolith_noise(world*3.0+vec2(13.2,4.7));
    if(medium_vis>0.0) medium=regolith_noise(rotation*world*10.0+vec2(7.4,1.3));
    if(fine_vis>0.0) fine=regolith_noise(transpose(rotation)*world*20.0+vec2(2.1,8.8));
    if(micro_vis>0.0) micro=regolith_noise(micro_rotation*world*40.0+vec2(5.9,14.3));
    vec2 gradient=transpose(rotation)*broad.yz*(0.028*0.15)*broad_vis
        +rotation*drift.yz*(0.27*0.032)*drift_vis
        +coarse.yz*(3.0*0.0023)*coarse_vis
        +transpose(rotation)*medium.yz*(10.0*0.0016)*medium_vis
        +rotation*fine.yz*(20.0*0.0010)*fine_vis
        +transpose(micro_rotation)*micro.yz*(40.0*0.00042)*micro_vis;
    // world.y is north, whereas the renderer's Z axis points south.
    vec3 n=normalize(surface_normal+surface_normal.y*vec3(-gradient.x,0.0,gradient.y));

    float mineral=noise(world/730.0)*0.7+noise(world/270.0+17.2)*0.3;
    float slope=clamp((1.0-surface_normal.y)*4.0,0.0,1.0);
    vec3 dust=pow(vec3(122.,111.,96.)/255.0,vec3(2.2));
    vec3 regolith=pow(vec3(103.,93.,81.)/255.0,vec3(2.2));
    vec3 pale=pow(vec3(145.,132.,111.)/255.0,vec3(2.2));
    vec3 basalt=pow(vec3(69.,68.,63.)/255.0,vec3(2.2));
    vec3 slate=pow(vec3(85.,92.,92.)/255.0,vec3(2.2));
    vec3 albedo=mix(regolith,dust,smoothstep(0.22,0.8,mineral));
    albedo=mix(albedo,pale,0.3*smoothstep(0.64,0.85,mineral));
    albedo=mix(albedo,mix(basalt,slate,mineral),slope*0.7);
    float strata=sin(v_mesh.y*0.31+noise(world/85.0)*2.0);
    albedo *= 0.945+0.035*strata;
    // Rare, irregular bright/dark mineral flecks, not a repeated pebble cell.
    // Symmetric tails preserve the average and die off within 10–20 m.
    float fleck_field=0.5+0.65*(fine.x-0.5)+0.35*(micro.x-0.5)*micro_vis;
    float flecks=smoothstep(0.70,0.90,fleck_field)-smoothstep(0.70,0.90,1.0-fleck_field);
    // Add centred bands together rather than multiplying them, to avoid a
    // distance-dependent colour shift as correlated octaves fade out.
    albedo *= 1.0+0.12*(broad.x-0.5)*broad_vis
                 +0.10*(drift.x-0.5)*drift_vis
                 +0.07*(coarse.x-0.5)*coarse_vis
                 +0.11*(medium.x-0.5)*medium_vis
                 +0.19*(fine.x-0.5)*fine_vis
                 +0.18*(micro.x-0.5)*micro_vis
                 +0.22*flecks*fine_vis*(1.0-smoothstep(10.0,20.0,dist));
    // Exposed rock is darker than dust; retain the sunlit ridge without a
    // near-white strip against the otherwise subdued mineral palette.
    albedo *= 1.0-0.23*(1.0-smoothstep(0.40,0.85,surface_normal.y));

    float direct=max(dot(n,u_sun),0.0)*smoothstep(-0.02,0.02,u_sun.y)*u_sun_visibility*shadowing(surface_normal);
    float reflected=max(dot(n,u_vhal),0.0)*max(u_vhal.y,0.0);
    // A very small, zero-mean cavity term affects ambient light only.
    float micro_ao=1.0+0.08*(coarse.x-0.5)*coarse_vis
                      +0.10*(medium.x-0.5)*medium_vis
                      +0.09*(fine.x-0.5)*fine_vis;
    vec3 illumination=(vec3(0.158,0.182,0.232)
        +vec3(0.115,0.220,0.270)*u_vhal_light*(0.35+0.65*reflected))*micro_ao
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

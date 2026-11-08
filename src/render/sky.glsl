#version 300 es
precision highp float;
precision highp int;

in vec2 v_ndc;
out vec4 frag_color;

// Fixed eight-body contract: Kesh, Vhal, Ossa, Thren, Sable, Nadir, Cinder, Sur.
// All distances remain camera-relative kilometres, including shadow ray origins.
uniform vec4 u_spheres[8]; // xyz center, w radius
uniform vec4 u_axes[8]; // xyz spin axis, w differential rotation angle
uniform vec4 u_materials[8]; // linear RGB tint, w enabled
uniform float u_kinds[8];
uniform vec3 u_forward, u_right, u_up;
uniform vec2 u_lens; // tan(FOVy / 2), aspect
uniform vec3 u_ring_center, u_ring_normal;
uniform vec4 u_ring; // inner radius, outer radius, brightness, enabled
uniform int u_surface, u_octaves, u_ring_steps;
uniform float u_exposure, u_haze, u_vhal_light;

const float PI = 3.14159265359;
const float FAR = 1.0e30;

float hash13(vec3 p) {
    p = fract(p * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}

float noise3(vec3 p) {
    vec3 i = floor(p), f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(mix(hash13(i), hash13(i + vec3(1,0,0)), f.x),
                   mix(hash13(i + vec3(0,1,0)), hash13(i + vec3(1,1,0)), f.x), f.y),
               mix(mix(hash13(i + vec3(0,0,1)), hash13(i + vec3(1,0,1)), f.x),
                   mix(hash13(i + vec3(0,1,1)), hash13(i + vec3(1,1,1)), f.x), f.y), f.z);
}

float fbm(vec3 p) {
    float value = 0.0, amplitude = 0.5, total = 0.0;
    // Rotating the domain between octaves avoids axial/checkerboard artifacts.
    const mat3 rotation = mat3(0.00,0.80,0.60, -0.80,0.36,-0.48, -0.60,-0.48,0.64);
    for (int i = 0; i < 5; ++i) {
        if (i >= u_octaves) break;
        value += amplitude * noise3(p);
        total += amplitude;
        p = rotation * p * 2.03 + vec3(17.1, 9.2, 3.7);
        amplitude *= 0.5;
    }
    return value / total;
}

// Stable near roots: compute perpendicular separation instead of subtracting
// two squared astronomical distances (essential for subpixel moons and Kesh).
float sphere_near(vec3 origin, vec3 direction, vec4 sphere) {
    vec3 delta = sphere.xyz - origin;
    float along = dot(delta, direction);
    vec3 perpendicular = delta - direction * along;
    float disc = sphere.w * sphere.w - dot(perpendicular, perpendicular);
    if (disc < 0.0 || along < 0.0) return FAR;
    float t = along - sqrt(disc);
    return t > 0.0 ? t : FAR;
}

// Finite-star penumbra for just the visible surface, never all eight surfaces.
// Also used on the rings, so transits/eclipses follow the same world geometry.
float sunlight(vec3 point, vec3 light_dir, float light_distance, int self_id) {
    float visibility = 1.0;
    float star_angle = u_spheres[0].w / max(light_distance, 1.0);
    for (int i = 1; i < 8; ++i) {
        if (i == self_id || u_materials[i].w < 0.5) continue;
        vec3 delta = u_spheres[i].xyz - point;
        float along = dot(delta, light_dir);
        if (along <= 0.0 || along >= light_distance) continue;
        float separation = length(delta - light_dir * along);
        float penumbra = max(2.0, along * star_angle * 0.30);
        // Exact ray-sphere test handles the umbra. The analytic distance adds
        // a narrow antialiased finite-source transition around its edge.
        float hit = sphere_near(point, light_dir, u_spheres[i]);
        float soft = smoothstep(-penumbra, penumbra, separation - u_spheres[i].w);
        if (hit < light_distance || separation < u_spheres[i].w + penumbra)
            visibility = min(visibility, soft);
    }
    return visibility;
}

float ring_profile(float radius, float footprint) {
    float span = max(u_ring.y - u_ring.x, 1.0);
    float x = (radius - u_ring.x) / span;
    float dx = max(footprint / span, 0.00003);
    float edges = smoothstep(-dx, dx, x) * (1.0 - smoothstep(1.0-dx, 1.0+dx, x));
    float density = 0.47 + 0.10 * sin(x * 31.0 + 0.9 * sin(x * 11.0))
                         + 0.065 * sin(x * 89.0 + sin(x * 23.0));
    // Finite dusty edge taper, while retaining the subpixel geometric edge.
    edges *= smoothstep(0.0, 0.025+dx, x) * (1.0-smoothstep(0.96-dx, 1.0, x));
    // Analytic low-pass filtering suppresses radial moire at oblique angles.
    density += 0.055 * sin(x * 417.0) * exp(-pow(dx * 417.0, 2.0));
    density += 0.025 * sin(x * 1193.0) * exp(-pow(dx * 1193.0, 2.0));
    // Three divisions, unequal in width/depth: the broad Cleave and two seams.
    density *= mix(0.06, 1.0, smoothstep(0.018-dx, 0.023+dx, abs(x-0.43)));
    density *= mix(0.27, 1.0, smoothstep(0.005-dx, 0.010+dx, abs(x-0.77)));
    density *= mix(0.38, 1.0, smoothstep(0.003-dx, 0.007+dx, abs(x-0.19)));
    return clamp(density * edges, 0.0, 1.0);
}

float ring_density(float radius, float footprint) {
    float result = 0.0;
    for (int i = 0; i < 4; ++i) {
        if (i >= u_ring_steps) break;
        float offset = (float(i) + 0.5) / float(u_ring_steps) - 0.5;
        result += ring_profile(radius + offset * footprint, footprint);
    }
    return result / float(u_ring_steps);
}

float ring_shadow(vec3 point, vec3 light, float light_distance) {
    float denom = dot(light, u_ring_normal);
    if (u_ring.w < 0.5 || abs(denom) < 0.0001) return 1.0;
    float t = dot(u_ring_center-point, u_ring_normal) / denom;
    if (t <= 2.0 || t >= light_distance) return 1.0;
    float radius = length(point + light*t - u_ring_center);
    float penumbra = max(90.0, t * u_spheres[0].w / light_distance * 0.22);
    float density = ring_density(radius, penumbra);
    return exp(-density * 1.5 / max(abs(denom), 0.20));
}

vec3 body_local(vec3 normal, int id) {
    vec3 axis = normalize(u_axes[id].xyz + vec3(0.0, 1e-9, 0.0));
    vec3 reference = abs(axis.y) < 0.9 ? vec3(0,1,0) : vec3(0,0,1);
    vec3 x = normalize(cross(reference, axis));
    vec3 z = cross(axis, x);
    vec3 p = vec3(dot(normal, x), dot(normal, axis), dot(normal, z));
    float angle = u_axes[id].w * (1.0 - 0.14 * p.y * p.y);
    float c = cos(angle), s = sin(angle);
    p.xz = mat2(c, -s, s, c) * p.xz;
    return p;
}

vec3 gas_clouds(vec3 p) {
    float latitude = asin(clamp(p.y, -1.0, 1.0));
    vec3 stretched = p * vec3(2.6, 10.0, 2.6);
    float warp = fbm(stretched + vec3(2.4,0.0,7.5)) - 0.5;
    float lat = latitude + 0.052 * warp;
    // Three broad zones with eleven subordinate belts; no longitude seam.
    float broad = 0.5 + 0.5 * sin(lat * 6.0 + 0.5);
    float belts = sin(lat * 22.0 + warp * 2.3 + 0.6 * sin(lat*6.0));
    float filaments = fbm(stretched * vec3(2.0, 4.1, 2.0) + warp*2.0);
    float value = clamp(0.37 + broad * 0.38 + belts * 0.13 + (filaments-0.5)*0.18, 0.0, 1.0);
    vec3 color = mix(vec3(0.205,0.160,0.119), vec3(0.650,0.593,0.465), value);
    float polar = smoothstep(0.70, 0.96, abs(p.y));
    return mix(color, vec3(0.485,0.537,0.514)*(0.88+filaments*0.17), polar * 0.72);
}

vec3 solid_surface(vec3 p, int id) {
    float continents = fbm(p*5.0 + vec3(3.1,7.2,1.4));
    float detail = fbm(p*54.0 + continents*2.0);
    if (u_kinds[id] < 2.5) {
        // Ossa: faded mineral flats, slate and basalt, not a red/Mars palette.
        vec3 dust = mix(vec3(0.136,0.109,0.084), vec3(0.283,0.235,0.164),
                        smoothstep(0.25,0.72,continents));
        dust = mix(dust, vec3(0.091,0.099,0.097), smoothstep(0.60,0.78,continents)*0.66);
        return dust * (0.83 + detail*0.35);
    }
    vec3 color = u_materials[id].rgb * (0.65 + continents*0.46);
    if (u_kinds[id] > 3.5 && u_kinds[id] < 4.5)
        color = mix(color, vec3(0.64,0.69,0.69), smoothstep(0.44,0.65,continents)*0.42);
    return color * (0.84 + detail*0.28);
}

vec3 shade_body(vec3 rd, int id, float t) {
    vec4 sphere = u_spheres[id];
    vec3 point = rd*t;
    vec3 normal = normalize(point-sphere.xyz);
    float facing = max(0.0, dot(normal, -rd));
    if (u_kinds[id] < 0.5) {
        // Hard photosphere, a hint of limb darkening, no bloom/flare texture.
        // Keep the photosphere below the ACES white plateau so its limb
        // darkening survives the display transform, even at high exposure.
        return vec3(1.12,1.38,1.78) * (0.38 + 0.62*facing);
    }
    vec3 to_light = u_spheres[0].xyz-point;
    float light_distance = length(to_light);
    vec3 light = to_light / light_distance;
    float nl = dot(normal, light);
    vec3 local = body_local(normal, id);
    vec3 albedo = u_kinds[id] < 1.5 ? gas_clouds(local) : solid_surface(local, id);
    float visibility = sunlight(point + normal*max(0.02,sphere.w*0.00008), light, light_distance, id);
    visibility *= ring_shadow(point + normal*max(0.02,sphere.w*0.00008), light, light_distance);
    // Layered 2–3 degree terminator. Belts remain legible across twilight.
    float terminator = smoothstep(-0.025, 0.026, nl);
    float direct = (pow(max(nl,0.0),0.72)*0.93 + 0.018*terminator) * visibility;
    vec3 fill = vec3(0.010,0.017,0.021);
    if (id != 1) {
        vec3 to_vhal = normalize(u_spheres[1].xyz-point);
        float vhal_solid_angle = pow(u_spheres[1].w / max(length(u_spheres[1].xyz-point),u_spheres[1].w),2.0);
        fill += vec3(0.13,0.22,0.25) * u_vhal_light * vhal_solid_angle * max(0.0,dot(normal,to_vhal));
    } else {
        // A cold, very dim ring-plane bounce, not a blue self-lit globe.
        float plane_fill = pow(1.0-abs(dot(normal,u_ring_normal)),3.0);
        // After albedo, this gives roughly 0.01--0.03 linear radiance:
        // enough for barely legible night belts after ACES, far below daylight.
        fill += vec3(0.008,0.012,0.014);
        fill += vec3(0.025,0.038,0.043) * (0.50+0.50*plane_fill) * u_ring.z;
        fill += vec3(0.006,0.010,0.013) * pow(1.0-facing,3.0);
    }
    vec3 color = albedo * (vec3(0.83,0.91,1.05)*direct + fill);
    if (u_kinds[id] < 1.5) {
        float rim = exp(-facing*26.0);
        color += vec3(0.10,0.17,0.20) * rim * (0.20+0.80*terminator) * (0.25+0.75*visibility);
    }
    return color;
}

vec3 starfield(vec3 rd) {
    // Homogeneous jittered 3D cells intersect a thin spherical shell, then
    // project radially. No face selection, angular seam, poles or stretched
    // cells. 4*pi*32^2*0.04 ~= 515 stars over the whole celestial sphere.
    // Neighbour evaluation preserves complete kernels across cell boundaries.
    vec3 base = floor(rd*32.0);
    vec3 result = vec3(0.0);
    float pixel = max(length(fwidth(rd))*0.42,0.00008);
    for (int z=-1; z<=1; ++z) {
        for (int y=-1; y<=1; ++y) {
            for (int x=-1; x<=1; ++x) {
                vec3 cell = base+vec3(float(x),float(y),float(z));
                float seed = hash13(cell+vec3(47.1,13.7,91.3));
                if (seed >= 0.04) continue;
                vec3 point = cell+vec3(hash13(cell+17.3),hash13(cell+39.1),hash13(cell+71.9));
                float radius = length(point);
                if (abs(radius-32.0)>0.5) continue;
                vec3 star_dir = point/radius;
                float magnitude = hash13(cell+vec3(19.3,67.1,2.7));
                float sigma = mix(0.00022,0.00065,pow(magnitude,3.0));
                float variance = sigma*sigma+pixel*pixel;
                vec3 offset = rd-star_dir;
                float core = exp(-0.5*dot(offset,offset)/variance);
                float energy = sigma*sigma/variance;
                vec3 tint = mix(vec3(0.58,0.72,1.0),vec3(0.92,0.95,1.0),magnitude);
                result += tint*core*energy*(0.18+1.8*magnitude*magnitude);
            }
        }
    }
    return result;
}

vec3 atmosphere(vec3 rd, vec3 sun_dir) {
    float altitude = max(rd.y,0.0);
    float haze = max(u_haze,0.0);
    float band = exp(-altitude/(0.065+0.045*haze));
    float dust = exp(-abs(rd.y)*27.0);
    float forward = pow(max(dot(rd,sun_dir),0.0),28.0);
    vec3 color = vec3(0.0014,0.0024,0.0045);
    color += vec3(0.024,0.039,0.047)*band*haze;
    color += vec3(0.004,0.006,0.007)*dust*haze;
    color += vec3(0.018,0.026,0.033)*forward*haze*(0.25+0.75*band);
    return color;
}

vec3 aces(vec3 x) {
    return clamp((x*(2.51*x+0.03))/(x*(2.43*x+0.59)+0.14),0.0,1.0);
}
vec3 linear_to_srgb(vec3 x) {
    return mix(12.92*x, 1.055*pow(max(x,vec3(0.0)),vec3(1.0/2.4))-0.055, step(vec3(0.0031308),x));
}

void main() {
    vec3 rd = normalize(u_forward + v_ndc.x*u_lens.x*u_lens.y*u_right + v_ndc.y*u_lens.x*u_up);
    vec3 sun_dir = normalize(u_spheres[0].xyz);
    vec3 sky = u_surface == 1 ? atmosphere(rd,sun_dir) : vec3(0.0006,0.0010,0.0020);
    float horizon = exp(-max(rd.y,0.0)*9.0);
    float star_transmission = exp(-max(u_haze,0.0)*(0.08+3.0*horizon));
    vec3 color = sky + starfield(rd) * (u_surface == 1 ? star_transmission : 1.0);
    float sun_angle = asin(clamp(u_spheres[0].w/length(u_spheres[0].xyz),0.0,0.999));
    float angular_separation = atan(length(cross(rd,sun_dir)),dot(rd,sun_dir));
    float off_disc = max(0.0,angular_separation-sun_angle);
    if (u_materials[0].w > 0.5) {
        // Thin analytic corona; rapidly vanishes outside a fraction of a degree.
        color += vec3(0.13,0.19,0.28)*exp(-off_disc/0.0018);
        if (u_surface == 1) color += vec3(0.007,0.011,0.017)*exp(-off_disc/0.026)*u_haze;
    }

    int nearest_id = -1;
    float nearest_t = FAR, coverage = 0.0;
    for (int i = 0; i < 8; ++i) {
        vec4 sphere = u_spheres[i];
        vec3 perpendicular = sphere.xyz-rd*dot(sphere.xyz,rd);
        float separation = length(perpendicular);
        float edge = separation-sphere.w;
        float aa = max(fwidth(edge)*0.70,0.01);
        // Camera inside a sphere: do not shade its interior or its back side.
        if (u_materials[i].w < 0.5 || dot(sphere.xyz,sphere.xyz) <= sphere.w*sphere.w) continue;
        float along = dot(sphere.xyz,rd);
        float t = along-sqrt(max(0.0,sphere.w*sphere.w-separation*separation));
        if (along > 0.0 && edge < aa && t > 0.0 && t < nearest_t) {
            nearest_id = i;
            nearest_t = t;
            coverage = 1.0-smoothstep(-aa,aa,edge);
        }
    }
    if (nearest_id >= 0) {
        vec3 surface = shade_body(rd,nearest_id,nearest_t);
        if (u_surface == 1) {
            float extinction = clamp(u_haze*(0.012+0.23*horizon),0.0,0.45);
            surface = mix(surface,sky,extinction);
        }
        color = mix(color,surface,coverage);
    }

    // Ray-plane ring hit, sorted against the nearest opaque sphere. A back
    // ring cannot paint through the planet, any moon, or the stellar disc.
    float denom = dot(rd,u_ring_normal);
    float ring_t = abs(denom)>1e-6 ? dot(u_ring_center,u_ring_normal)/denom : -1.0;
    vec3 ring_point = rd*ring_t;
    float ring_radius = length(ring_point-u_ring_center);
    float ring_footprint = max(fwidth(ring_radius)*0.65,2.0);
    if (u_ring.w>0.5 && ring_t>0.0 && (ring_t<nearest_t || nearest_id<0)
        && ring_radius>u_ring.x-ring_footprint && ring_radius<u_ring.y+ring_footprint) {
        float density = ring_density(ring_radius,ring_footprint);
        float opacity = 1.0-exp(-density*0.20/max(abs(denom),0.18));
        vec3 to_light = u_spheres[0].xyz-ring_point;
        float light_distance = length(to_light);
        vec3 light = to_light/light_distance;
        float lit = sunlight(ring_point,light,light_distance,-1);
        float scatter = pow(max(dot(rd,light),0.0),6.0)*0.16;
        float illumination = (0.17+0.12*abs(dot(light,u_ring_normal))+scatter*0.70)*lit;
        // Palette is linearized icy grey (#c9cec8 / #4b5453), then lit.
        // Brightness is applied once, preserving the full 0..1.5 control range.
        vec3 ice = vec3(0.584,0.617,0.578)*(0.78+0.12*density);
        vec3 shadow = vec3(0.070,0.089,0.087)*0.20;
        vec3 ring_color = (shadow+ice*illumination)*u_ring.z;
        if (u_surface==1) ring_color = mix(ring_color,sky,clamp(u_haze*(0.012+0.23*horizon),0.0,0.45));
        color = mix(color,ring_color,opacity);
    }
    color = linear_to_srgb(aces(max(color,vec3(0.0))*clamp(u_exposure,0.6,1.6)));
    color += (hash13(vec3(gl_FragCoord.xy,47.0))-0.5)/255.0;
    frag_color = vec4(clamp(color,0.0,1.0),1.0);
}

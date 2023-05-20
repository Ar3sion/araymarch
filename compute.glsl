#version 430

layout(local_size_x = 16, local_size_y = 16) in;

layout(location = 0) uniform mat4 cameraTransformation;

layout(rgba32f, binding = 0) uniform image2D imageOutput;

const float FOV = 1.57079632679f;
const float FOCAL_DISTANCE = 1.0f / tan(FOV * 0.5f);

const float MAX_STEPS = 1000.0f;
const float INTERSECTION_DISTANCE = 1e-6;
const float LOD_RATIO = 0.0007f;
const float MAX_T = 50.0f;

const vec3 LIGHT_DIRECTION = normalize(vec3(0.5f, 0.7f, 0.5f));
const float MIN_SHADOW_DARKNESS = 0.3;

float sponge(vec3 z) {
    float w = 2.0f;
    for (int n = 0; n < 5; n++) {
        z = abs(z);
        if (z.x < z.y) z.xy = z.yx;
        if (z.x < z.z) z.xz = z.zx;
        if (z.y < z.z) z.yz = z.zy;
        z *= 3.0f;
        w *= 3.0f;
        z.xyz -= vec3(2.0f, 2.0f, 2.0f);
        if(z.z < -1.0f)
            z.z += 2.0f;
    }
    return length(max(abs(z.xyz) - vec3(1.0), 0.0)) / w;
}

float distanceFunction(vec3 position) {
    return sponge(position / 10.0f) * 10.0f;
}

struct RaymarchOutput {
    float softDistance;
    float steps;
    bool intersection;
};

RaymarchOutput raymarch(inout vec3 position, vec3 direction) {
    float t = 0.0f;
    float softDistance = 1e38;
    bool intersection = false;
    float steps = 0.0f;
    for(; steps < MAX_STEPS; steps += 1.0f) {
        float distance = distanceFunction(position);
        float minDistance = max(LOD_RATIO * t, INTERSECTION_DISTANCE);
        if(distance < minDistance){
            return RaymarchOutput(0.0f, steps, true);
        }
        if(t > MAX_T){
            return RaymarchOutput(softDistance, steps, false);
        }
        softDistance = min(softDistance, distance / t);
        t += distance;
        position += distance * direction;
    }
    return RaymarchOutput(0.0f, steps, true);
}

//Differentiate by sampling a tetrahedron https://www.iquilezles.org/www/articles/normalsSDF/normalsSDF.htm
vec3 calculateNormal(vec3 position) {
    return normalize(
        vec3(1.0f, -1.0f, -1.0f) * distanceFunction(position + vec3(1.0f, -1.0f, -1.0f) * INTERSECTION_DISTANCE) +
        vec3(-1.0f, -1.0f, 1.0f) * distanceFunction(position + vec3(-1.0f, -1.0f, 1.0f) * INTERSECTION_DISTANCE) +
        vec3(-1.0f, 1.0f, -1.0f) * distanceFunction(position + vec3(-1.0f, 1.0f, -1.0f) * INTERSECTION_DISTANCE) +
        vec3(1.0f, 1.0f, 1.0f) * distanceFunction(position + vec3(1.0f, 1.0f, 1.0f) * INTERSECTION_DISTANCE)
    );
}

vec3 computePixel(vec2 position) {
    vec3 rayPosition = (cameraTransformation * vec4(0.0f, 0.0f, 0.0f, 1.0f)).xyz;
    vec3 rayDirection = mat3(cameraTransformation) * normalize(vec3(position, -FOCAL_DISTANCE));
    
    RaymarchOutput raymarchOutput = raymarch(rayPosition, rayDirection);
    if(raymarchOutput.intersection) {
        vec3 normal = calculateNormal(rayPosition);
        
        vec3 shadowRayPosition = rayPosition;
        shadowRayPosition += normal * INTERSECTION_DISTANCE; //So that the object doesn't cast a shadow on itself
        RaymarchOutput shadowRaymarchOutput = raymarch(shadowRayPosition, LIGHT_DIRECTION);
        float k = clamp(5.0f * shadowRaymarchOutput.softDistance, MIN_SHADOW_DARKNESS, 1.0f);
        k += 0.2 * (dot(normal, LIGHT_DIRECTION) + 1.0f); //Diffuse
        k += (1.0 / (1.0 + raymarchOutput.steps * 0.01f) - 1.0f) * 0.7f; //Occlusion    
        
        return vec3(0.5f, 0.7f, 1.0f) * k;
    } else {
        return vec3(1.0f, 1.0f, 1.0f);
    }
}

void main() {
    ivec2 coords = ivec2(gl_GlobalInvocationID);
    ivec2 size = imageSize(imageOutput);
    if(coords.x < size.x && coords.y < size.y) {
        vec2 position = (vec2(coords) / vec2(size)) * 2.0f - 1.0f;
        position.y = -position.y;
        position.x *= float(size.x) / float(size.y);
        vec4 pixel = vec4(computePixel(position), 1.0f);
        imageStore(imageOutput, coords, pixel);
    }
}
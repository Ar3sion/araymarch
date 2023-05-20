#version 430

in vec2 textureCoordinates;

layout(binding = 0) uniform sampler2D quadTexture;

out vec4 fragment;

void main() {
    fragment = texture(quadTexture, textureCoordinates);
}

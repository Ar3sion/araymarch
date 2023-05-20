#version 430

layout(location = 0) in vec2 position;

out vec2 textureCoordinates;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    textureCoordinates = vec2(position.x, -position.y) * 0.5f + 0.5f;
}

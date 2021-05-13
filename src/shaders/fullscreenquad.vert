#version 450
layout(location = 0) out vec2 fragTexCoord;

const vec2 positions[3] = vec2[3](
	vec2(-1.0, -1.0),
	vec2(3.0, -1.0),
	vec2(-1.0, 3.0)
);

void main() {
	vec2 position = positions[gl_VertexIndex];
	gl_Position = vec4(position, 0.0, 1.0);
	fragTexCoord = (position+vec2(1.0, 1.0))/2.0;
}
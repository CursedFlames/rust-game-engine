#version 450
layout(location = 0) in vec3 position;
layout(location = 1) in vec2 uv;
// layout(location = 2) in ivec4 tint;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec3 color;

layout(set=0, binding=0) uniform Uniforms {
	mat4 transform;
};

void main() {
	vec4 pos4 = vec4(position.xyz, 1.0);
	vec4 transformed_pos = transform * pos4;
	gl_Position = transformed_pos;
	fragTexCoord = uv;
	color = vec3(transformed_pos.xy, 1.0-(transformed_pos.x + transformed_pos.y)/2.0);
}
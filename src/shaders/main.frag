#version 450
layout(location = 0) in vec2 fragTexCoord;
layout(location = 1) in vec3 color;

layout(location = 0) out vec4 f_color;

//layout(push_constant) uniform PushConstants {
//	float time;
//} pushConstants;

//layout(binding = 0) uniform sampler2D texSampler;

void main() {
//	 f_color = vec4(sin(pushConstants.time/4.0), 0.25, 1.0, 1.0);
//	f_color = texture(texSampler, fragTexCoord);
	f_color = vec4(0.8, 0.2, 0.6, 1.0);
}
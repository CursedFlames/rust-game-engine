#version 450
layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 f_color;

//layout(push_constant) uniform PushConstants {
//	float time;
//} pushConstants;

layout(set = 0, binding = 0) uniform texture2D t_1;
layout(set = 0, binding = 1) uniform sampler s_1;

void main() {
	// f_color = vec4(sin(pushConstants.time/4.0), 0.25, 1.0, 1.0);
	f_color = texture(sampler2D(t_1, s_1), fragTexCoord);
//	f_color = vec4(1.0-fragTexCoord.x, fragTexCoord.x, 0.0, 1.0);
}
//
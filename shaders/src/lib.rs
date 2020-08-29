pub mod vs {
	vulkano_shaders::shader! {
		ty: "vertex",
		src: "\
#version 450
layout(location = 0) in vec3 position;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec3 color;

layout(push_constant) uniform PushConstants {
	// not needed but I left it here because I didn't feel like fiddling with offsets
	float time;
	mat4 transform;
} pushConstants;

void main() {
	vec4 pos4 = vec4(position.xyz, 1.0);
	vec4 transformed_pos = pushConstants.transform*pos4;
	gl_Position = transformed_pos;
	fragTexCoord = position.xy;
	color = vec3(transformed_pos.xy, 1.0-(transformed_pos.x + transformed_pos.y)/2.0);
}"
	}
}

pub mod vs_output {
	vulkano_shaders::shader! {
		ty: "vertex",
		src: "\
#version 450
layout(location = 0) in vec2 position;

layout(location = 0) out vec2 fragTexCoord;

void main() {
	gl_Position = vec4(position, 0.0, 1.0);
	fragTexCoord = (position+vec2(1.0, 1.0))/2.0;
}"
	}
}

pub mod fs_triangle {
	vulkano_shaders::shader! {
		ty: "fragment",
		src: "\
#version 450
layout(location = 0) in vec2 fragTexCoord;
layout(location = 1) in vec3 color;

layout(location = 0) out vec4 f_color;

layout(push_constant) uniform PushConstants {
	float time;
} pushConstants;

void main() {
	// f_color = vec4(sin(pushConstants.time/4.0), 0.25, 1.0, 1.0);
	f_color = vec4(color, 1.0);
}"
	}
}

pub mod fs_output {
	vulkano_shaders::shader! {
		ty: "fragment",
		src: "\
#version 450
layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 f_color;

layout(push_constant) uniform PushConstants {
	float time;
} pushConstants;

layout(binding = 0) uniform sampler2D texSampler;

void main() {
	// f_color = vec4(sin(pushConstants.time/4.0), 0.25, 1.0, 1.0);
	f_color = texture(texSampler, fragTexCoord);
}"
	}
}

#version 450

struct Material {
	vec3 colour;// 12
	uint id;// 16
	vec2 pos;// 24
	vec2 vel;// 32
	vec2 target;// 40
	float mass;// 44
	float force;// 48
	float stable;// 52
	uint tags;// 56
	uint gas;// 60
};

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

layout(binding = 0) buffer Data {
Material mat[];
}
buf;

void main() {
	uint idx = gl_GlobalInvocationID.x;
	buf.mat[idx].tags += 1;
	// buf.mat[idx].colour+=vec3(0.1);
}
#version 460

struct Material{
	uint id;
	vec3 colour;
	vec2 pos;
	vec2 vel;
	float mass;
	vec2 target;
	float force;
	float stable;
	uint tags;
	uint gas;
};

layout(local_size_x=64,local_size_y=1,local_size_z=1)in;

layout(binding = 0) buffer Data {
	Material mat[];
}buf;

void main(){
	uint idx=gl_GlobalInvocationID.x;
	buf.mat[idx].tags += 1; 
	// buf.mat[idx].colour+=vec3(0.1);
}
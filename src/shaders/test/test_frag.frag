#version 460

// layout(push_constant)uniform PushConstantData{
// 	vec2 screensize;
// }pc;
layout(location=0)out vec4 f_color;

void main(){
	vec2 uv=gl_FragCoord.xy/vec2(1920.,100.);
	f_color=vec4(uv.x,uv.y,0.,1.);
}
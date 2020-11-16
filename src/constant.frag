#version 450

precision highp int;

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) flat in int v_Index;  // dynamically non-uniform
layout(location = 0) out vec4 o_Color;

layout(set = 0, binding = 0) uniform utexture2D u_Textures[2];
layout(set = 0, binding = 1) uniform sampler u_Sampler;

void main() {
    if (v_Index == 0) {
        uint samp = texture(usampler2D(u_Textures[0], u_Sampler), v_TexCoord).r;
        float color_out = float(2080 - (int(samp) >> 4)) / 410.0;

        o_Color = vec4(vec3(color_out), 1.0);

        /*
        vec3 color = texture(sampler2D(u_Textures[0], u_Sampler), v_TexCoord).rgb;
        o_Color = vec4(mix(
            color,
            vec3( dot( color , vec3( 0.2126 , 0.7152 , 0.0722 ) ) ),
            0.5
        ), 1.0);
        */
    } else {
        // We need to write something to output color
        o_Color = vec4(0.0, 0.0, 1.0, 0.0);
    }
}

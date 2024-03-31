//
// Generated by Microsoft (R) D3DX9 Shader Compiler 9.08.299.0000
//
//   vsa shaderdump19/ISHIT2001.pso /Fcshaderdump19/ISHIT2001.pso.dis
//
//
//#define	ScreenSpace	Src0
// Parameters:

sampler2D ScreenSpace : register(s0);
float4 blurParams : register(c1);


// Registers:
//
//   Name         Reg   Size
//   ------------ ----- ----
//   blurParams   const_1       1
//   ScreenSpace         texture_0       1
//

// Structures:

struct VS_OUTPUT {
    float2 ScreenOffset : TEXCOORD0;
};

struct PS_OUTPUT {
    float4 color_0 : COLOR0;
};

// Code:

float3 pSamples[] ={
                         {-0.929317,-0.295797,1},
                         {0.525823,0.672132,1},
                         {0.867314,0.132608,1},
                         {0.002469,-0.692395,1},
                         {-0.273576,0.914886,1},
                         {-0.729743,0.451020,1},
                         {0.544711,-0.537214,1},
                         {-0.607088,-0.736707,1},
	                     {-0.737059,-0.816707,1}
					 };

                     
PS_OUTPUT main(VS_OUTPUT IN) {
    PS_OUTPUT OUT;

    const float4 constant_vec = {1, 0.2, 0.2, 0}; //r0
    float4 col = tex2D(ScreenSpace, IN.ScreenOffset.xy); 
    
    float3 retCol = 0;
	float weights = 0;
    float2 blur_vec = (blurParams.xy * constant_vec.y) + IN.ScreenOffset.yx - (blurParams.yx * constant_vec.x);
	for (int i=0;  i < 9;  i++ )
	{
		retCol += tex2D(ScreenSpace, (IN.ScreenOffset +  blur_vec + pSamples[i].xy)).rgb;
		weights += pSamples[i].z;
	}
	retCol /= weights;

    OUT.color_0.a = 1;
    OUT.color_0.rgb = lerp(retCol.rgb, constant_vec.rgb, saturate((2 * length(IN.ScreenOffset.xy - 0.5)) - 0.5) * blurParams.z);

    return OUT;
};


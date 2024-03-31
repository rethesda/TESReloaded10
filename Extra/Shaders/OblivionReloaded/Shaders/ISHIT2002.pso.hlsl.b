//
// Generated by Microsoft (R) D3DX9 Shader Compiler 9.08.299.0000
//
//   vsa shaderdump19/ISHIT2002.pso /Fcshaderdump19/ISHIT2002.pso.dis
//
//
//#define	ScreenSpace	Src0
// Parameters:

sampler2D ScreenSpace : register(s0);
sampler2D OverlaySpace : register(s1);
float4 blurParams : register(c1);
float4 doubleVisParams : register(c2);


// Registers:
//
//   Name            Reg   Size
//   --------------- ----- ----
//   blurParams      const_1       1
//   doubleVisParams const_2       1
//   ScreenSpace            texture_0       1
//   Src1            texture_1       1
//

// Structures:

struct VS_OUTPUT {
    float2 ScreenSpace : TEXCOORD0;
    float2 OverlayOffset : TEXCOORD1;
    float2 ScreenPos : VPOS;
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
#define	expand(v)		(((v) - 0.5) / 0.5)
#define	compress(v)		(((v) * 0.5) + 0.5)
#define	weight(v)		dot(v, 1)
#define	sqr(v)			((v) * (v))

    float1 q1;
    float1 q2;
    float4 r1;
    float4 r2;
    const float4 constant_vec = {1, 0.2, 0.2, 0}; 
    float4 pointInOverlay = tex2D(OverlaySpace, IN.OverlayOffset.xy); //t3
    
    float2 r0 = IN.ScreenSpace.xy - doubleVisParams.xy;
    r1.y = max(r0.y, 1 - doubleVisParams.w);
    r1.x = max(r0.x, 0);
    r2.xy = IN.ScreenSpace.xy + doubleVisParams.xy;
    r0.y = min(r2.y, 1);
    r0.x = min(doubleVisParams.z, r2.x);
    r2.xyzw = tex2D(ScreenSpace, r1.xy);
    r1.xyzw = tex2D(ScreenSpace, r0.xy);
    q1.x = 2 * ((doubleVisParams.z / doubleVisParams.w) * (IN.OverlayOffset.x - 0.5));	// [0,1] to [-1,+1]
    q2.x = min(sqrt(sqr(q1.x) + (expand(IN.ScreenSpace.y) * expand(IN.OverlayOffset.y))) * (5* blurParams.z), 1);
    OUT.color_0 = float4(0.5 * ((r2.xyz + r1.xyz) * (1 - q2.x)) + (pointInOverlay.xyz * q2.x), 0.7);
    
    float2 center = float2(0.5f, 0.5f);
    float2 camp_point = float2(0.5f, 0.1f);  //to check proper hardcoding values
    float distc = abs(camp_point.y - center.y);
    float2 dist_comp  = abs(center - IN.ScreenSpace);
    float  dist =  sqrt((dist_comp.x * dist_comp.x) + (dist_comp.y * dist_comp.y));
    float halo_intensity = blurParams.z * 3;
    OUT.color_0 = lerp((OUT.color_0) * (1 - (dist * halo_intensity)), float4(1,0, 0, 0.3) * (dist * halo_intensity), dist);
    OUT.color_0.a = saturate(dist);
    return OUT;
};

// approximately 11 instruction slots used (1 texture, 10 arithmetic)

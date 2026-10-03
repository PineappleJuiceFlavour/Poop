// GTA side: draws Just Cause 3 wherever JC3 is nearer than GTA (both depths in metres).
// GTA depth settings (Edit global preprocessor definitions):
//   RESHADE_DEPTH_INPUT_IS_REVERSED=1, RESHADE_DEPTH_LINEARIZATION_FAR_PLANE=1000
#include "ReShade.fxh"

texture JC3ColorTex : JC3COLOR;
texture JC3DepthTex : JC3DEPTH;
sampler sColor { Texture = JC3ColorTex; AddressU = CLAMP; AddressV = CLAMP; };
sampler sDepth { Texture = JC3DepthTex; MinFilter = POINT; MagFilter = POINT; AddressU = CLAMP; AddressV = CLAMP; };

uniform bool JC3Active = false; // set by the add-on
uniform float MaxDist = 80.0;   // set by the add-on from tuning.csv
uniform float DepthBias < ui_type = "drag"; ui_min = 0.0; ui_max = 2.0; ui_label = "Depth bias (m)"; > = 0.05;
uniform float EdgeFade < ui_type = "drag"; ui_min = 0.0; ui_max = 20.0; ui_label = "Fade out before max distance (m)"; > = 10.0;
uniform int DebugView < ui_type = "combo"; ui_items = "Composite\0GTA depth\0JC3 depth\0"; > = 0;

float3 PS_Composite(float4 pos : SV_Position, float2 uv : TEXCOORD) : SV_Target {
    float3 gta = tex2D(ReShade::BackBuffer, uv).rgb;
    float gd = ReShade::GetLinearizedDepth(uv) * RESHADE_DEPTH_LINEARIZATION_FAR_PLANE;
    if (DebugView == 1) return frac(gd).xxx;
    if (!JC3Active) return gta;
    float jd = tex2D(sDepth, uv).r;
    if (DebugView == 2) return frac(jd).xxx;
    if (jd > MaxDist || jd > gd + DepthBias) return gta;
    float fade = saturate((MaxDist - jd) / max(EdgeFade, 0.001));
    return lerp(gta, tex2D(sColor, uv).rgb, fade);
}

technique JC3Passthrough < ui_tooltip = "Composites Just Cause 3 into GTA 5 (JC3xGTA5)."; > {
    pass { VertexShader = PostProcessVS; PixelShader = PS_Composite; }
}

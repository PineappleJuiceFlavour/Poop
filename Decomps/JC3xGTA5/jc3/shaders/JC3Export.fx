// JC3 side: packs JC3's picture and depth (metres) into JC3OutTex for the JC3xGTA5 add-on to export.
// Set ReShade's depth settings for JC3 (Edit global preprocessor definitions):
//   RESHADE_DEPTH_INPUT_IS_REVERSED=1, RESHADE_DEPTH_LINEARIZATION_FAR_PLANE=1000
#include "ReShade.fxh"

texture JC3OutTex { Width = BUFFER_WIDTH; Height = BUFFER_HEIGHT; Format = RGBA32F; };

float4 PS_Pack(float4 pos : SV_Position, float2 uv : TEXCOORD) : SV_Target {
    float3 c = tex2D(ReShade::BackBuffer, uv).rgb;
    float d = ReShade::GetLinearizedDepth(uv);
    float metres = d >= 0.999 ? 1e9 : d * RESHADE_DEPTH_LINEARIZATION_FAR_PLANE; // sky never composites
    return float4(c, metres);
}

technique JC3Export < ui_tooltip = "Exports JC3 to GTA 5 (JC3xGTA5). Keep it last."; > {
    pass { VertexShader = PostProcessVS; PixelShader = PS_Pack; RenderTarget = JC3OutTex; }
}

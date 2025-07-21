// https://gist.github.com/nxrighthere/eb208dae8b66dbe452af223f276e46cc

// AGX Tonemapping - Converted from Unreal Engine implementation
// Original by Missing Deadlines (Benjamin Wrensch)
// Based on Troy Sobotka's AgX: https://github.com/sobotka/AgX

// AGX Look options: 0 = Default, 1 = Golden, 2 = Punchy
const AGX_LOOK: i32 = 0;

// Mean error^2: 3.6705141e-06
fn agxDefaultContrastApprox(x: vec3<f32>) -> vec3<f32> {
    let x2 = x * x;
    let x4 = x2 * x2;
    
    return 15.5 * x4 * x2
         - 40.14 * x4 * x
         + 31.96 * x4
         - 6.868 * x2 * x
         + 0.4298 * x2
         + 0.1191 * x
         - 0.00232;
}

fn agx(val: vec3<f32>) -> vec3<f32> {
    let agx_mat = mat3x3<f32>(
        vec3<f32>(0.842479062253094, 0.0423282422610123, 0.0423756549057051),
        vec3<f32>(0.0784335999999992, 0.878468636469772, 0.0784336),
        vec3<f32>(0.0792237451477643, 0.0791661274605434, 0.879142973793104)
    );
    
    // DEFAULT_LOG2_MIN      = -10.0
    // DEFAULT_LOG2_MAX      =  +6.5
    // MIDDLE_GRAY           =  0.18
    // log2(pow(2, VALUE) * MIDDLE_GRAY)
    // Adjusted for Unreal's zero exposure compensation
    let min_ev = -12.47393; // Default: -12.47393
    let max_ev = 0.526069;  // Default:  4.026069
    
    // Input transform (inset)
    var result = agx_mat * val;
    
    // Log2 space encoding
    result = clamp(log2(result), vec3<f32>(min_ev), vec3<f32>(max_ev));
    result = (result - min_ev) / (max_ev - min_ev);
    
    // Apply sigmoid function approximation
    result = agxDefaultContrastApprox(result);
    
    return result;
}

fn agxEotf(val: vec3<f32>) -> vec3<f32> {
    let agx_mat_inv = mat3x3<f32>(
        vec3<f32>(1.19687900512017, -0.0528968517574562, -0.0529716355144438),
        vec3<f32>(-0.0980208811401368, 1.15190312990417, -0.0980434501171241),
        vec3<f32>(-0.0990297440797205, -0.0989611768448433, 1.15107367264116)
    );
    
    // Inverse input transform (outset)
    var result = agx_mat_inv * val;
    
    // sRGB IEC 61966-2-1 2.2 Exponent Reference EOTF Display
    // NOTE: We're linearizing the output here. Comment/adjust when
    // *not* using a sRGB render target
    result = pow(result, vec3<f32>(2.2));
    
    return result;
}

fn agxLook(val: vec3<f32>) -> vec3<f32> {
    let lw = vec3<f32>(0.2126, 0.7152, 0.0722);
    let luma = dot(val, lw);
    
    // Default values
    let offset = vec3<f32>(0.0, 0.0, 0.0);
    var slope = vec3<f32>(1.0, 1.0, 1.0);
    var power = vec3<f32>(1.0, 1.0, 1.0);
    var sat = 1.0;
    
    if (AGX_LOOK == 1) {
        // Golden
        slope = vec3<f32>(1.0, 0.9, 0.5);
        power = vec3<f32>(0.8, 0.8, 0.8);
        sat = 0.8;
    } else if (AGX_LOOK == 2) {
        // Punchy
        slope = vec3<f32>(1.0, 1.0, 1.0);
        power = vec3<f32>(1.35, 1.35, 1.35);
        sat = 1.4;
    }
    
    // ASC CDL
    let result = pow(val * slope + offset, power);
    return luma + sat * (result - luma);
}

fn apply_tone_mapping(linearColorRec709: vec3<f32>) -> vec3<f32> {
    var result = agx(linearColorRec709);
    result = agxLook(result);
    result = agxEotf(result);
    
    return result;
}


// Uncharted 2 Filmic Tonemapping
fn filmicToneMapping(color: vec3<f32>) -> vec3<f32> {
    let A = 0.15; // Shoulder strength
    let B = 0.50; // Linear strength
    let C = 0.10; // Linear angle
    let D = 0.20; // Toe strength
    let E = 0.02; // Toe numerator
    let F = 0.30; // Toe denominator
    
    return ((color * (A * color + C * B) + D * E) / (color * (A * color + B) + D * F)) - E / F;
}

fn apply_tone_mapping(color: vec3<f32>) -> vec3<f32> {
    let exposureBias = 2.0;
    let curr = filmicToneMapping(exposureBias * color);
    
    let W = 11.2; // Linear white point value
    let whiteScale = 1.0 / filmicToneMapping(vec3<f32>(W));
    
    return curr * whiteScale;
}

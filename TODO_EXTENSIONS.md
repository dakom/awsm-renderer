## Goal

1. Implement the ior, transmision, and volume extensions
2. Update the specular extension (and any other code paths) to account for ior

## Preperation already completed

Everything is avilable in `PbrMaterial` in `material.wgsl`, there should be no further changes needed on the Rust side, all changes are confined to the shader code.

## Considerations

Both opaque and transparent materials can have these properties. The logic for handling these properties should be integrated into the existing material system without disrupting the current functionality.

If it helps, refactoring code into `shared_wgsl/*` is allowed to keep things clean and modular.

Call-out any assumptions you make or decisions you take that might affect other parts of the codebase.

Some existing code, particularly `specular` extension code, will be affected by these new changes, especially ior

## References

These references should be read in full, as they contain important details about how the extensions should be implemented and interact with each other.

- Ior: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_ior
- Transmission: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_transmission
- Volume: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_volume
- Specular: https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_specular

## Test Scenes

We have these setup to test in the renderer, I can provide feedback on how they look once implemented.

- Ior: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareIor
- Transmission: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareTransmission
- Volume: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareVolume
- Specular: https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/CompareSpecular

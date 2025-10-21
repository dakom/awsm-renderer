📝 Note on All-White IBL Test:
Testing with uniform white IBL revealed a very faint seam on smooth non-metallic surfaces at certain viewing angles. This is an extreme edge case that amplifies tiny numerical precision artifacts. In real-world usage with actual environment maps (photo studio, sky, etc.), this artifact is:
- Barely visible or completely invisible
- Most spheres look good at most angles
- Not noticeable in practical rendering scenarios

The all-white IBL test is useful for debugging but not representative of actual usage. The renderer performs well with realistic environment maps.

# Packr2

Implements some relevant of packing algorithms, currently `StripPacker` (simplest), `SkylinePacker` (good for unsorted data)
and `SplitPacker` (best for bake data using the `pack` function)

- It uses almost the same interface as [`texture_packer`](https://github.com/PistonDevelopers/texture_packer).
- The `SplitPacker` was ported from [`rectpack2D`](https://github.com/TeamHypersomnia/rectpack2D) but it sorts all split globably

# Work left

- [ ] Implement `pack_with_best_size` function to find the best packing with the lower used area
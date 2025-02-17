Demo created over the weekend of [MountainBytes](https://www.mountainbytes.ch/) 2025, as part of their DemoLab workshop. This is my first demo, and second Bevy project.

The aim is to visually depict a thought as a high-dimensional oscillation. I was inspired to draw an analogy between feature vectors in machine learning models, eigenvectors of a matrix, and specifically normal modes in oscillation problems. The spherical harmonic functions, as an infinite orthonormal basis, seemed like a good set of functions to combine to create an interesting oscillating shape.

I was further inspired by Cristoph Peter's [work on raytracing spherical harmonic glyphs](https://momentsingraphics.de/VMV2023.html) for displaying MRI diffusion data, though in this case I rasterised an icosphere rather than using any of his code.

The main challenges were learning how to drive shaders in Bevy and pass data in through a storage buffer, etc., as well as computing normals within the vertex shader for the heavy displacement. I could definitely do more (e.g. credits text would be a nice addition) but this was all I had time to do during the party.

I did not put a lot of effort into sizecoding beyond [the basics](https://github.com/johnthagen/min-sized-rust) of stripping the Bevy binary and turning on a few size optimisations in the Rust compiler. It would likely be possible to get this binary much smaller with more knowledge.

The brain is derived from an MRI scan provided by Ronja.

The music is 'Cute' by [ALPA](https://demozoo.org/sceners/59844/), CC BY-NC-SA 4.0.

This work is shared under a [CC-BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/deed.en) license.

Thanks for taking a look at my demo, I can't wait to make more!

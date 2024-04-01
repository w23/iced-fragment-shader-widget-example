# Custom fragment shader widget example for iced

A minimalistic example of making [iced](https://iced.rs/) custom shader widget draw its contents using just a fragment shader. This provides reasonably fast access to writing arbitrary pixels into widget's canvas, something that iced custom canvas widget struggles with.

This example consists of:
- Iced boilerplate code for creating, updating, and drawing custom shader widget.
- [Wgpu](https://wgpu.rs/) pipeline creation for a simple single-triangle pipeline without any buffers for vertex data.
- [Wgsl](https://www.w3.org/TR/WGSL/) shader file including both vertex and fragment shader code.
    - vertex shader generates 3 vertices coordinates based on `vertex_index` to make a triangle that fills the entire viewport.
    - fragment shader draws a pannable/zoomable mandelbrot set as a trivial example
- Mouse event handling that updates widget state. This state is then passed into the shader as uniform data.

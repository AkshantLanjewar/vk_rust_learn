# Learn Vulkan Rust Tutorial

## Purpose

This repository serves as both my personal learning through the rust vulkan tutorial,
that can be found [here](https://kylemayes.github.io/vulkanalia/), so that I can
learn the basics around utilizing rust to render out computer graphics as opposed
to OpenGL (hint, its a bitch of a lot more work), as well as a post tutorial breakdown,
on how to take the learnings, and create a real base for a rendering engine for
any 3d graphics project to take as its core, and build up around, designed around
the strengths and ways the vulkan API works.

## Ground Covered in Tutorial

In terms of actual material that was covered during the tutorial, the sample in this
repository is able to render a single model, that can be instanced utilizing the
left or right arrow keys, with:

- **LEFT KEY:** decrement the number of rendered models
- **RIGHT KEY:** increment the number of rendered models

With the minimum amount of models that are displayed being one. This is quite a simple
sample, more focused on the technical aspects of getting started rendering with
vulkan, ranging from:

- **Basics:** Setting up the vulkan device and instance, command buffers, pipeline
  to render out a triangle
- **Using Uniforms + Push Constants:** How to utilize both uniforms, as well as
  push constants (as well as setting them up), to upload data from the CPU to the
  shader for its vertex calculations.
- **Models + Texturing:** How to load in a model, render it with vulkan, and
  apply its textures when being rendered. Additionally there are peices related
  to mipmap creation, and multisampling.
- **Basic Dynamic Scenes:** How to recycle command buffers so that they can be
  updated to change the scene, and how to utilize secondary command buffers so that
  more complicated scenes can be rendered in a single render pass.

Unfortunately no material or code is within this repo for rendering objects with
different shaders (or even the basics of materials), as those can be found in
OpenGL and DirectX tutorials, which can simply be ported over to the vulkan API.

## Post-OP Content

The purpose of the post-op is to act as a explainer, on the sample itself, and as a
starting template on how to take the knowledge learned from the explainer, and turn
it into a simple rendering engine, that can be built up with more advanced features
down the line without too much hassle.

The rendering-engine architecture designed out in the post-op section is not meant
to act as a starting point for any cross platform rendering engine (rendering API
agnostic), but rather tailored to how vulkan specifically is best optimized to render
out computer graphics.

**NOTE:** Within the rendering engine there is no discussion of other "game engine"
systems such as audio, or UI, apart from input handling and scene management.
Implementation of those is left to be added as an additional layer on top of the
rendering engine, not as a part of it.


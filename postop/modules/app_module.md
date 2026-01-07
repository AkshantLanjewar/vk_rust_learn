# App Module

**Location:** `src/app.rs`

## Overview

### Description

- The app module defines the app structure, which is the main structure that sets up
  and moves around the vulkan resources in order to render out the sample of the
  loaded spinning objects.
- It is a static and brittle structure, defined to solely render the single scene, and
  serves as a entrypoint for all the vulkan based functionality that is found within
  the application.

### Purpose

- The app module is a central location for all the vulkan logic, and serves as the entry
  point to tie together all the different bits into a single structure that can both
  initialize and then render frame by frame the demo scene.

## Symbols Defined

### `App`

#### Members

- `entry:` This is the vulkan library entry that is loaded in for vulkanalia
- `instance:` This is the vulkan instance that is created by the app on initialization
- `device:` This is the vulkan device, that the application selected that will be the
  actual device handling the rendering commands
- `data:` These are all the vulkan data structures created from both the instance and the
  device that are utilized for rendering.
- `frame:` This is the current frame that is being rendered [value from 0 to 1 since 
  only 2 max frames in flight]
- `resized:` Flag that gets set to true when the window frame gets resized
- `start:` This is the timestamp when the application was first initialized, use to setup
  internal tick system
- `models:` This is the number of models that should be rendered in the demo scene, to
  help demo the usage of secondary command buffers for dynamic rendering.

#### Implementations

##### `pub unsafe fn create(window: &Window) -> Result<Self>`

**Params:**

- `window:` the reference to the winit window to create the vulkan application for

**Description:** This is the function that initializes a new instance of the application
struct, given a reference to a Winit window (to setup the vulkan rendering resources
properly)

**Process:**

1. Load and setup the vulkan library into vulkanalia
2. Create a vulkan instance given the window
3. Pick a physical device using the instance and the window
4. Create the logical device from the picked physical device
5. Create all the resources needed to render the scene:

    1. create swapchain
    2. create pipeline
    3. create command pool
    4. create framebuffer
    5. create texture and sampler
    6. load the model and vertex data into GPU memory
    7. create uniform buffers and descriptor sets for dynamic data
    8. record the command buffers
    9. record syncronization objects

6. return the created app instance

##### `unsafe fn update_secondary_command_buffer`

**Params:**

- `&mut self:` This is a method that requires a mutable instance of the app struct
- `image_index:` The index of the vulkan image (frame) that we will be rendering the
  output to
- `model_index:` In our demo scene, we can render 1 to 4 objects, this is the index
  of the object that this secondary command buffer is rendering (affects the positioning
  of the object in the global rendering coords)

**Description:**

- In order to be able to chunk up our rendering calls into multiple command buffers,
  than have a mega command buffer where all the draw calls are made from, we can utilize
  secondary command buffers, which this function updates based on the number of objects
  that are being rendered in the demo scene.
- Thus for each model that we render, we can issue the commands to render it in a single
  secondary buffer. This allows the commands in this secondary command buffer to utilize
  the render pass of the main command buffer that will be rendered, and can be just added
  or easily removed form the command buffer when needed.
- Though the use of secondary command buffers are quite simple in this example, but at
  its core it allows for the easy implenetation of dynamic scenes, as secondary command
  buffers can be kept alive on their own, and have their own lifetimes independent
  of the actual command buffer they are used in, allowing for a lot of dynamic calling
  potential.

**Process:**

1. Ensure the secondary Command buffers are present for the current frame that is
   being rendered too.
2. If there is no secondary command buffer for the model, create one
3. setup the model matrix and get bytes form of model
4. begin the secondary command buffer
5. bind the pipeline used to render out the geometry
6. bind the vertex, index buffers, descriptor sets, push constants
7. draw the geometry with the indexed command and end the command buffer
8. Return the command buffer

##### `unsafe fn update_command_buffer(&mut self, image_index: usize) -> Result<()>`

**Params:**

- `&mut self:` This is a method that requires a mutable instance of the app struct
- `image_index:` This is the current frame that we are updating the command buffer
  for (next frame after the current displayed one)

**Description:**

- when vulkan notifies the next image that will be presented to the screen is ready to
  be drawn, we update the command buffer for that image, taking into account changes
  in scene variables, such as the number of models we are rendering, to update our
  primary command buffer, which sends the draw commands for the next frame to be displayed
  in the window.
- The method in which we re-use our primary command buffers is by resetting the command
  buffer, clearing all the old draw commands from it, allowing us to then write our new
  draw commands for the frame.
- The function updates the command buffers in-place (aka where they are stored within the
  buffer list), and they are then executed by the render function.

**Process:**

1. Reset the command buffer for the given image
2. begin the render pass with the render area and clear value
3. collect the secondary command buffers based on the number of models that will be
   rendered in this given instance
4. end the render pass and command buffer

##### `pub unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()>`

**Params:**

- `&mut self:` This is a method that requires a mutable instance of the app struct
- `window:` the reference to the winit window to recreate the swapchain for

**Description:**

- The swapchain is a queue of images that are waiting to be presented to the window
  in which our application is given a image (the image index) to render too, and then
  return back to the queue to be presented.
- Vulkan forces us to create this infrastructure on our own, and thus we abstract away
  the creation of the swapchain into its own function, since it is dependent on the
  conditions of the window we are rendering too.
- This function is called on the initialization of the application, as well as when the
  window is resized, to make sure the swapchain infrastructure matches the state of the
  window at all times.

**Process:**

1. wait till the vulkan device is done with its current in-progress work
2. destroy the vertex data as well as the swap chain
3. create the swapchain and the pipeline again
4. create the window objects
5. create the vertex and index buffers
6. create uniform buffers
7. create descriptor sets
8. create command buffers
9. resize the images in flight to the swapchain images length

##### `unsafe fn destroy_swapchain(&mut self)`

**Params:**

- `&mut self:` This is a method that requires a mutable instance of the app struct

**Description:**

- this function bundles all the functions needed to cleanup the swapchain that we have
  created to present images within the application, including resources that rely on the
  swapchain for their creation such as the pipeline or command buffers
- This function is used in both the overall cleanup for the application, as well as
  when the window is resized and the swapchain is re-created.

**Process:**

1. destroy the color and depth images
2. destroy the descriptor pool
3. destroy the uniform buffers
4. destroy the framebuffers
5. destroy the command pool
6. destroy the pipeline resources
7. destroy the render pass
8. destroy swapchain and the swapchain images

##### `pub unsafe fn render(&mut self, window: &Window) -> Result<()>`

**Params:**

- `&mut self:` This is a method that requires a mutable instance of the app struct
- `window: &Window:` This is the window we are rendering the frame too

**Description:**

- This is the function that is called to render a new frame to the window that is given
  to the application, transforming the internal set of render state into draw calls,
  and then real pixels that are displayed on the window this application renders too

**Process:**

1. wait for the fence of the current frame
2. acquire the next image from the swapchain
3. if there is a fence for the next image, wait for the fence
4. update the command and uniform buffers
5. reset the fences for the current image we are rendering too
6. submit the draw info to the graphics queue
7. present the result to the presentation queue
8. if the window was resized or the swapchain internally changes re-create the swapchain
9. increment the frame

##### `pub unsafe fn destroy(&mut self)`

**Params:**

- `&mut self:` This is a method that requires a mutable instance of the app struct

**Description:**

- This is the function that cleans up all of the vulkan resources for a safe exit from
  the application

**Process:**

1. destroy the swapchain
2. destroy texture 
3. destroy the destcriptor set
4. destroy vertex and index buffer memory
5. destroy the synchronization objects
6. destroy the command pool
7. destroy the validation layers
8. destroy the surface and the instance

### `AppData`

This is the structure that holds the rest of the vulkan objects. For the purposes of
clarity, we wont describe any of the members here, as there are too many to properly
keep track off, but rather, as they are created and use, reference them back to the
appData struct, so that you can get a better idea of references and usages.

#### References (TODO)


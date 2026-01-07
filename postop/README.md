# Post-OP Index

## Description

The POST-OP of this tutorial is split into 2 phases:

1. **Sample Understanding:** How this whole vulkan toy sample works, how all the pieces
   fit together to produce the scene that is rendered out during vulkan. During this
   phase each piece is talked about in some detail as to give a general understanding,
   as well as its usage understanding together for better knowledge.
2. **Transformation into Rendering Engine:** The second phase discusses how all the
   pieces talked about in the above phase, can be organized into a system where
   dynamic scenes can be constructed, rendered, and moved around, with systems
   such as materials, lighting, basic geometric primitives, model loading,
   per-object shaders, etc... All the basics you would need to get a 3d application
   up and running utilizing rust and vulkan, in a manner so that you can build
   your app around the rendering, not the rendering around the app.

## Sample Understanding

When first analyzing the `src/` folder of this repository, we are seen with 2 rust
files in the root, as well as 3 further modules where code is located:

- `foundation:` The basics needed to get a vulkan app up and running
- `pipeline:` All the vulkan related code related to graphics pipelines and rendering
  geometry
- `scenes:` The basic files covering the basic dynamic scene features such as model
  loading, msaa sampling, and mip-mapping.

In addition to the two main modules:

- `app.rs`: the home of the App struct, which wraps all of the vulkan objects and
  functionality into a single rust struct, handles vulkan initialization, rendering
  and cleanup
- `main.rs:` The home of the entrypoint, where both the window which the application
  will be rendered too is setup, as well as where the App struct is initialized,
  and called in loop to render to the window screen.

### Entrypoint Breakdown

**Location:** `src/main.rs`

**Description:**

- The main entrypoint is more of a wrapper around the Appilcation struct, which is where
  all of our vulkan code is located, as well as the winit window that the application
  renders too.
- the first set of code, lines `19:28` relate to setting up the external pre-requisites
  that are required to create a vulkan rendering application, namely the pretty env
  logger which helps us handle the more tedious bits of vulkan based logging, as well
  as the winit event loop and window, creating the initial window for our vulkan app
  to render too.
- afterwards, we create the instance of our actual vulkan application itself, on line
  `32`, and then runs the event loop, from which the application state is handled
  based on the events that are passed to the winit window:

    - **AboutToWait:** This is a window event that triggers the window to render a
      new frame since the window is about to come into focus.
    - **WindowEvent::RedrawRequested:** This event first checks that the window is not
      exiting, as well if the window is focused. If both of those are true then it
      calls the `App::render` method, in order to have the application render a new
      frame to the window it is given.
    - **WindowEvent::CloseRequested:** This is the event that is fired when the user
      closes the window using the OS's windowing manager. It exits out of the event
      loop for the window, as well as destroys the apps internal vulkan resources
      so that it exits out in a safe fashion.
    - **WindowEvent::Resized:** This is the event that is fired when the user resizes
      the window with their operating system, where the block of code sets an internal
      flag within the application structure, to notify the application to re-create
      vulkan objects for the new frame size.
    - **WindowEvent::KeyboardInput:** This is the event that handles taking in basic
      keyboard input that was activated while the window was focused. The events that
      are handled are arrow left and right, updating an internal application value
      to control how many instances of the model are going to be rendered.

### App Breakdown

**Location:** `src/app.rs`

**Description:**

- The application module is a crucial module that relates to being the entrypoint for all
  of the application, through its two main structs, `App` and `AppData`, where `App` is
  the actual object (in the vulkan sense) that handles running the core rendering loop,
  setting up resources, manaing them etc... while `AppData` is the struct that contains
  a bulk of the vulkan objects that are created, with exception of the `Instance` and
  `Device`, which can be considered the parent objcets from which all the objects in
  the `AppData` would be created from.
- The `App` struct implements a couple of key functions that are utilized through
  the application in the `main` module:

  - `create(window: &Window) -> Result<Self>:` This is the function that creates the
    initial vulkan resources to render and setup the app to render 3d scenes to the
    winit window that was passed to it.
  - `update_secondary_command_buffer:` This is the function that updates the secondary
    command buffer used to render the loaded object, so that it can be spun around
    in a dynamic fashion.
  - `update_command_buffer:` This is the function that handles updating the command
    buffer for the current frame that is to be rendered.
  - `recreate_swapchain:` This is the function that recreates the swapchain resources
    whenever a window is resized or initialized for the first time.
  - `destroy_swapchain:` This is the function that destroys all the vulkan resources
    held by the application in relation to the swapchain, which is how we tell the window
    to update and render new frames.
  - `render:` This is the function that handles rendering out a new frame for the window
    that is passed to it, updating both the command and uniform buffers with the current
    commands needed to draw out the frame that was requested. Additionally it handles
    the device window synchronization to make sure the correct frame is rendered and
    then displayed.
  - `destroy:` This is the function that handles destroying all of the resources the
    application has created for both rendering out the scene, as well as utilizing
    vulkan, doing a full clean so no used memory stays allocated.

- the details of the `AppData` struct are quite long, and can be found within the detailed
  breakdown for the application module.

Additional details on the app module can be found [here](./modules/app_module.md)

### Foundation Breakdown

**Location:** `src/foundation/mod.rs`

**Description:**

- The foundation module contains the key modules that help setup the core of a vulkan
  application, including the device and instance, as well as the swapchain, aka the
  core pieces to have a vulkan application, but not actually anything related to
  core rendering.
- this module acts more as a caller library, where the functions exposed are mainly
  called from the application module, but the nature of these resources allow for
  an opportunity to create isolated objects in a real production application.

Additional details on the foundation module, and its submodules can be found [here]()

### Pipeline Breakdown

**Location:** `src/pipeline/mod.rs`

**Description:**

- The pipeline module is the module that is responsible for all the functions related
  to creating and utilizing a vulkan graphics pipeline to render out the "scene" of
  4 textured houses. Unlike in opengl, the graphics pipeline is not dynaically changeable
  but static based on how the pipeline is configured on its creation.
- In our example, we only created a single pipeline, and thus all the pipeline functions
  were simplified around that fact. However, in a real scenario you might want to utilize
  different shaders or other items for different objects that are to be rendered, and
  thus will need a more general way to create and use multiple pipelines through the
  rendering system.
- The module exposes buffers, descriptors, etc... all submodules on pieces that either
  are utilized by the pipeline or use the pipeline.
- Additionally the module exposes 2 functions in the root of the module definition:

  - `create_render_pass:` The render pass is an object that is filled with the 
    attachments, or image views that the final image should use when constructing
    an image, like color and depth buffers, with instructions on how to handle them
    during the rendering, how many samples, etc...
  - `create_pipeline:` This is the function that handles creating the graphics
    pipeline, from which all rendering operations reference, assembling the
    vert and frag shaders as well as the options selected, storing the pipeline
    in `AppData.pipeline`

Additional details on the pipeline module, and its submodules can be found [here]()

### Scenes Breakdown

**Location:** `src/scenes/mod.rs`

**Description:**

- The scenes module is a higher level module that is built in to load more commonly
  shared features such as model loading, generating mipmaps for textures, as well
  as texture sampling / loading.
- It exports out 3 submodules, mipmaps, models, and sampling, all building upon
  the lower level features found in the pipeline and foundation modules.

Additional details on the scenes module, and its submodules can be found [here]()

## Render Engine Transformation

**TODO:** Not Started


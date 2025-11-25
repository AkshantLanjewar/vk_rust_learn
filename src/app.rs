use cgmath::{Deg, vec3};
use std::time::Instant;
use std::u64;

use anyhow::{Result, anyhow};
use vulkanalia::loader::{LIBRARY, LibloadingLoader};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::{
    ExtDebugUtilsExtensionInstanceCommands, KhrSurfaceExtensionInstanceCommands,
    KhrSwapchainExtensionDeviceCommands,
};
use vulkanalia::window as vk_window;
use vulkanalia::{Entry, window};

use winit::window::Window;

use crate::foundation::device::{create_logical_device, pick_physical_device};
use crate::foundation::instance::{VALIDATION_ENABLED, create_instance};
use crate::foundation::swapchain::{create_swapchain, create_swapchain_image_views};
use crate::pipeline::buffers::{create_command_buffers, create_command_pool, create_framebuffers};
use crate::pipeline::descriptors::{
    Mat4, create_descriptor_pool, create_descriptor_set_layout, create_descriptor_sets,
    create_uniform_buffers, update_uniform_buffer,
};
use crate::pipeline::image::{create_depth_objects, create_texture_image};
use crate::pipeline::render::create_sync_objects;
use crate::pipeline::texture::{create_texture_image_view, create_texture_sampler};
use crate::pipeline::vertex::{Vertex, create_index_buffer, create_vertex_buffer};
use crate::pipeline::{create_pipeline, create_render_pass};
use crate::scenes::models::load_model;
use crate::scenes::sampling::create_color_objects;

#[derive(Clone, Debug)]
pub struct App {
    entry: Entry,
    instance: Instance,
    pub device: Device,
    data: AppData,
    frame: usize,
    pub resized: bool,
    pub start: Instant,
    pub models: usize,
}

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

impl App {
    /// Creates the vulkan application
    pub unsafe fn create(window: &Window) -> Result<Self> {
        let loader = unsafe { LibloadingLoader::new(LIBRARY)? };
        let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
        let mut data = AppData::default();

        let instance = unsafe { create_instance(window, &entry, &mut data)? };

        unsafe {
            data.surface = vk_window::create_surface(&instance, &window, &window)?;
            pick_physical_device(&instance, &mut data)?;
        }

        let device = unsafe { create_logical_device(&entry, &instance, &mut data)? };

        unsafe {
            create_swapchain(window, &instance, &device, &mut data)?;
            create_swapchain_image_views(&device, &mut data)?;

            create_render_pass(&instance, &device, &mut data)?;
            create_descriptor_set_layout(&device, &mut data)?;
            create_pipeline(&device, &mut data)?;
            create_command_pool(&instance, &device, &mut data)?;

            create_color_objects(&instance, &device, &mut data)?;
            create_depth_objects(&instance, &device, &mut data)?;
            create_framebuffers(&device, &mut data)?;

            create_texture_image(&instance, &device, &mut data)?;
            create_texture_image_view(&device, &mut data)?;
            create_texture_sampler(&device, &mut data)?;

            load_model(&mut data)?;
            create_vertex_buffer(&instance, &device, &mut data)?;
            create_index_buffer(&instance, &device, &mut data)?;

            create_uniform_buffers(&instance, &device, &mut data)?;
            create_descriptor_pool(&device, &mut data)?;
            create_descriptor_sets(&device, &mut data)?;

            create_command_buffers(&device, &mut data)?;
            create_sync_objects(&device, &mut data)?;
        }

        Ok(Self {
            entry,
            instance,
            device,
            data,
            frame: 0,
            resized: false,
            start: Instant::now(),
            models: 1,
        })
    }

    unsafe fn update_secondary_command_buffer(
        &mut self,
        image_index: usize,
        model_index: usize,
    ) -> Result<vk::CommandBuffer> {
        self.data
            .secondary_command_buffers
            .resize_with(image_index + 1, Vec::new);
        let command_buffers = &mut self.data.secondary_command_buffers[image_index];

        while model_index >= command_buffers.len() {
            let allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(self.data.command_pool)
                .level(vk::CommandBufferLevel::SECONDARY)
                .command_buffer_count(1);

            let command_buffer = self.device.allocate_command_buffers(&allocate_info)?[0];
            command_buffers.push(command_buffer);
        }

        let command_buffer = command_buffers[model_index];

        let y = (((model_index % 2) as f32) * 2.5) - 1.25;
        let z = (((model_index / 2) as f32) * -2.0) + 1.0;

        let time = self.start.elapsed().as_secs_f32();
        let model = Mat4::from_translation(vec3(0.0, y, z))
            * Mat4::from_axis_angle(vec3(0.0, 0.0, 1.0), Deg(90.0) * time);
        let model_bytes =
            std::slice::from_raw_parts(&model as *const Mat4 as *const u8, size_of::<Mat4>());

        let opacity = (model_index + 1) as f32 * 0.25;
        let opacity_bytes = &opacity.to_ne_bytes()[..];

        let inheritance_info = vk::CommandBufferInheritanceInfo::builder()
            .render_pass(self.data.render_pass)
            .subpass(0)
            .framebuffer(self.data.framebuffers[image_index]);

        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE)
            .inheritance_info(&inheritance_info);

        self.device.begin_command_buffer(command_buffer, &info)?;

        self.device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.data.pipeline,
        );
        self.device
            .cmd_bind_vertex_buffers(command_buffer, 0, &[self.data.vertex_buffer], &[0]);
        self.device.cmd_bind_index_buffer(
            command_buffer,
            self.data.index_buffer,
            0,
            vk::IndexType::UINT32,
        );
        self.device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.data.pipeline_layout,
            0,
            &[self.data.descriptor_sets[image_index]],
            &[],
        );
        self.device.cmd_push_constants(
            command_buffer,
            self.data.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            model_bytes,
        );
        self.device.cmd_push_constants(
            command_buffer,
            self.data.pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            64,
            &0.25f32.to_ne_bytes()[..],
        );
        self.device
            .cmd_draw_indexed(command_buffer, self.data.indices.len() as u32, 1, 0, 0, 0);

        self.device.end_command_buffer(command_buffer)?;
        Ok(command_buffer)
    }

    /// # Safety
    /// This is a vulkan using function and thus is unsafe
    unsafe fn update_command_buffer(&mut self, image_index: usize) -> Result<()> {
        let command_buffer = self.data.command_buffers[image_index];

        self.device
            .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        self.device.begin_command_buffer(command_buffer, &info)?;

        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(self.data.swapchain_extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let depth_clear_value = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };

        let clear_values = &[color_clear_value, depth_clear_value];
        let info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.data.render_pass)
            .framebuffer(self.data.framebuffers[image_index])
            .render_area(render_area)
            .clear_values(clear_values);

        self.device.cmd_begin_render_pass(
            command_buffer,
            &info,
            vk::SubpassContents::SECONDARY_COMMAND_BUFFERS,
        );

        let secondary_command_buffer = (0..self.models)
            .map(|i| self.update_secondary_command_buffer(image_index, i))
            .collect::<Result<Vec<_>, _>>()?;

        self.device
            .cmd_execute_commands(command_buffer, &secondary_command_buffer[..]);

        self.device.cmd_end_render_pass(command_buffer);

        self.device.end_command_buffer(command_buffer)?;

        Ok(())
    }

    /// # Safety
    /// This is a vulkan using function and thus is unsafe
    pub unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()?;
            self.device.destroy_buffer(self.data.vertex_buffer, None);
            self.destroy_swapchain();

            // recreate the swapchain
            create_swapchain(window, &self.instance, &self.device, &mut self.data)?;
            create_swapchain_image_views(&self.device, &mut self.data)?;
            create_render_pass(&self.instance, &self.device, &mut self.data)?;
            create_descriptor_set_layout(&self.device, &mut self.data)?;
            create_pipeline(&self.device, &mut self.data)?;

            create_color_objects(&self.instance, &self.device, &mut self.data)?;
            create_depth_objects(&self.instance, &self.device, &mut self.data)?;
            create_framebuffers(&self.device, &mut self.data)?;
            create_vertex_buffer(&self.instance, &self.device, &mut self.data)?;
            create_index_buffer(&self.instance, &self.device, &mut self.data)?;

            create_uniform_buffers(&self.instance, &self.device, &mut self.data)?;
            create_descriptor_pool(&self.device, &mut self.data)?;
            create_descriptor_sets(&self.device, &mut self.data)?;
            create_command_buffers(&self.device, &mut self.data)?;
        }

        self.data
            .images_in_flight
            .resize(self.data.swapchain_images.len(), vk::Fence::null());

        Ok(())
    }

    unsafe fn destroy_swapchain(&mut self) {
        unsafe {
            self.device
                .destroy_image_view(self.data.color_image_view, None);
            self.device.free_memory(self.data.color_image_memory, None);
            self.device
                .destroy_image_view(self.data.color_image_view, None);

            self.device
                .destroy_image_view(self.data.depth_image_view, None);
            self.device.free_memory(self.data.depth_image_memory, None);
            self.device.destroy_image(self.data.depth_image, None);

            self.device
                .destroy_descriptor_pool(self.data.descriptor_pool, None);

            self.data
                .uniform_buffers
                .iter()
                .for_each(|b| self.device.destroy_buffer(*b, None));
            self.data
                .uniform_buffers_memory
                .iter()
                .for_each(|m| self.device.free_memory(*m, None));

            self.data
                .framebuffers
                .iter()
                .for_each(|f| self.device.destroy_framebuffer(*f, None));

            self.device
                .free_command_buffers(self.data.command_pool, &self.data.command_buffers);
            self.device.destroy_pipeline(self.data.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.data.pipeline_layout, None);
            self.device.destroy_render_pass(self.data.render_pass, None);
            self.data
                .swapchain_image_views
                .iter()
                .for_each(|v| self.device.destroy_image_view(*v, None));
            self.device.destroy_swapchain_khr(self.data.swapchain, None);
        }
    }

    /// renders the frame for the vulkan application
    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        self.device
            .wait_for_fences(&[self.data.in_flight_fences[self.frame]], true, u64::MAX)?;

        let image_result = self.device.acquire_next_image_khr(
            self.data.swapchain,
            u64::MAX,
            self.data.image_available_semaphore[self.frame],
            vk::Fence::null(),
        );

        let image_index = match image_result {
            Ok((image_index, _)) => image_index as usize,
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => return self.recreate_swapchain(window),
            Err(e) => return Err(anyhow!(e)),
        };

        if !self.data.images_in_flight[image_index as usize].is_null() {
            self.device.wait_for_fences(
                &[self.data.images_in_flight[image_index as usize]],
                true,
                u64::MAX,
            )?;
        }

        self.data.images_in_flight[image_index as usize] = self.data.in_flight_fences[self.frame];

        self.update_command_buffer(image_index)?;
        update_uniform_buffer(&self.device, &self.start, &self.data, image_index)?;

        let wait_semaphores = &[self.data.image_available_semaphore[self.frame]];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = &[self.data.command_buffers[image_index as usize]];
        let signal_semaphores = &[self.data.render_finished_semaphore[self.frame]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        self.device
            .reset_fences(&[self.data.in_flight_fences[self.frame]])?;

        self.device.queue_submit(
            self.data.graphics_queue,
            &[submit_info],
            self.data.in_flight_fences[self.frame],
        )?;

        let swapchains = &[self.data.swapchain];
        let image_indices = &[image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        let result = self
            .device
            .queue_present_khr(self.data.present_queue, &present_info);
        let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
            || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);

        if self.resized || changed {
            self.resized = false;
            self.recreate_swapchain(window)?;
        } else if let Err(e) = result {
            return Err(anyhow!(e));
        }

        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    /// destroys the vulkan app
    pub unsafe fn destroy(&mut self) {
        self.destroy_swapchain();
        self.device.destroy_sampler(self.data.texture_sampler, None);
        self.device
            .destroy_image_view(self.data.texture_image_view, None);

        self.device.destroy_image(self.data.texture_image, None);
        self.device
            .free_memory(self.data.texture_image_memory, None);

        self.device
            .destroy_descriptor_set_layout(self.data.descriptor_set_layout, None);
        self.device.destroy_buffer(self.data.index_buffer, None);
        self.device.free_memory(self.data.index_buffer_memory, None);
        self.device.destroy_buffer(self.data.vertex_buffer, None);
        self.device
            .free_memory(self.data.vertex_buffer_memory, None);

        // destroy the sync objects
        self.data
            .in_flight_fences
            .iter()
            .for_each(|f| self.device.destroy_fence(*f, None));
        self.data
            .render_finished_semaphore
            .iter()
            .for_each(|s| self.device.destroy_semaphore(*s, None));
        self.data
            .image_available_semaphore
            .iter()
            .for_each(|s| self.device.destroy_semaphore(*s, None));

        // destroy the command pool
        self.device
            .destroy_command_pool(self.data.command_pool, None);
        self.device.destroy_device(None);

        if VALIDATION_ENABLED {
            self.instance
                .destroy_debug_utils_messenger_ext(self.data.messenger, None);
        }

        self.instance.destroy_surface_khr(self.data.surface, None);
        self.instance.destroy_instance(None);
    }
}

/// the vulkan handles and associated properties utilized
/// by the vulkan application
#[derive(Clone, Debug, Default)]
pub struct AppData {
    pub surface: vk::SurfaceKHR,
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub render_pass: vk::RenderPass,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub secondary_command_buffers: Vec<Vec<vk::CommandBuffer>>,
    pub image_available_semaphore: Vec<vk::Semaphore>,
    pub render_finished_semaphore: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub mip_levels: u32,
    pub texture_image: vk::Image,
    pub texture_image_memory: vk::DeviceMemory,
    pub texture_image_view: vk::ImageView,
    pub texture_sampler: vk::Sampler,
    pub depth_image: vk::Image,
    pub depth_image_memory: vk::DeviceMemory,
    pub depth_image_view: vk::ImageView,
    pub msaa_samples: vk::SampleCountFlags,
    pub color_image: vk::Image,
    pub color_image_memory: vk::DeviceMemory,
    pub color_image_view: vk::ImageView,
}

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
    create_descriptor_pool, create_descriptor_set_layout, create_descriptor_sets,
    create_uniform_buffers, update_uniform_buffer,
};
use crate::pipeline::image::{create_depth_objects, create_texture_image};
use crate::pipeline::render::create_sync_objects;
use crate::pipeline::texture::{create_texture_image_view, create_texture_sampler};
use crate::pipeline::vertex::{create_index_buffer, create_vertex_buffer};
use crate::pipeline::{create_pipeline, create_render_pass};

#[derive(Clone, Debug)]
pub struct App {
    entry: Entry,
    instance: Instance,
    pub device: Device,
    data: AppData,
    frame: usize,
    pub resized: bool,
    pub start: Instant,
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
            create_framebuffers(&device, &mut data)?;
            create_command_pool(&instance, &device, &mut data)?;
            create_depth_objects(&instance, &device, &mut data)?;

            create_texture_image(&instance, &device, &mut data)?;
            create_texture_image_view(&device, &mut data)?;
            create_texture_sampler(&device, &mut data)?;

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
        })
    }

    pub unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
        self.device.device_wait_idle()?;
        self.device.destroy_buffer(self.data.vertex_buffer, None);
        self.destroy_swapchain();

        // recreate the swapchain
        create_swapchain(window, &self.instance, &self.device, &mut self.data)?;
        create_swapchain_image_views(&self.device, &mut self.data)?;
        create_render_pass(&self.instance, &self.device, &mut self.data)?;
        create_descriptor_set_layout(&self.device, &mut self.data)?;
        create_pipeline(&self.device, &mut self.data)?;

        create_framebuffers(&self.device, &mut self.data)?;
        create_vertex_buffer(&self.instance, &self.device, &mut self.data);
        create_index_buffer(&self.instance, &self.device, &mut self.data);

        create_uniform_buffers(&self.instance, &self.device, &mut self.data)?;
        create_descriptor_pool(&self.device, &mut self.data)?;
        create_descriptor_sets(&self.device, &mut self.data)?;
        create_command_buffers(&self.device, &mut self.data)?;

        self.data
            .images_in_flight
            .resize(self.data.swapchain_images.len(), vk::Fence::null());

        Ok(())
    }

    unsafe fn destroy_swapchain(&mut self) {
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
    pub image_available_semaphore: Vec<vk::Semaphore>,
    pub render_finished_semaphore: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub texture_image: vk::Image,
    pub texture_image_memory: vk::DeviceMemory,
    pub texture_image_view: vk::ImageView,
    pub texture_sampler: vk::Sampler,
    pub depth_image: vk::Image,
    pub depth_image_memory: vk::DeviceMemory,
    pub depth_image_view: vk::ImageView,
}

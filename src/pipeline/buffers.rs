use anyhow::{Ok, Result};
use cgmath::{Deg, vec3};
use vulkanalia::{
    Device, Instance,
    vk::{self, DeviceV1_0, Handle, HasBuilder},
};

use crate::{app::AppData, foundation::device::QueueFamilyIndices, pipeline::descriptors::Mat4};

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_framebuffers(device: &Device, data: &mut AppData) -> Result<()> {
    data.framebuffers = data
        .swapchain_image_views
        .iter()
        .map(|i| {
            let attachments = &[data.color_image_view, data.depth_image_view, *i];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(data.render_pass)
                .attachments(attachments)
                .width(data.swapchain_extent.width)
                .height(data.swapchain_extent.height)
                .layers(1);

            device.create_framebuffer(&create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(())
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_command_pool(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
    let info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(indices.graphics);

    data.command_pool = device.create_command_pool(&info, None)?;
    Ok(())
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_command_buffers(device: &Device, data: &mut AppData) -> Result<()> {
    unsafe {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(data.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(data.framebuffers.len() as u32);

        data.command_buffers = device.allocate_command_buffers(&allocate_info)?;
        data.secondary_command_buffers = vec![vec![]; data.swapchain_images.len()];
    }

    Ok(())
}

pub unsafe fn begin_onetime_command(
    device: &Device,
    data: &AppData,
) -> Result<(vk::CommandBuffer)> {
    let info = vk::CommandBufferAllocateInfo::builder()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(data.command_pool)
        .command_buffer_count(1);

    let command_buffer = device.allocate_command_buffers(&info)?[0];
    let info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    device.begin_command_buffer(command_buffer, &info)?;

    Ok(command_buffer)
}

pub unsafe fn end_onetime_command(
    device: &Device,
    data: &AppData,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    device.end_command_buffer(command_buffer)?;

    let command_buffers = &[command_buffer];
    let info = vk::SubmitInfo::builder().command_buffers(command_buffers);

    device.queue_submit(data.graphics_queue, &[info], vk::Fence::null())?;
    device.queue_wait_idle(data.graphics_queue)?;
    device.free_command_buffers(data.command_pool, &[command_buffer]);

    Ok(())
}

use anyhow::{Result, anyhow};
use std::fs::File;
use std::ptr::copy_nonoverlapping as memcpy;

use vulkanalia::{
    Device, Instance,
    vk::{self, DeviceV1_0, HasBuilder, InstanceV1_0},
};

use crate::{
    app::AppData,
    pipeline::{
        buffers::{begin_onetime_command, end_onetime_command},
        vertex::{create_buffer, get_memory_type_index},
    },
    scenes::mipmaps::generate_mipmaps,
};

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_image(
    instance: &Instance,
    device: &Device,
    data: &AppData,
    width: u32,
    height: u32,
    mip_levels: u32,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Image, vk::DeviceMemory)> {
    let info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(mip_levels)
        .array_layers(1)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::_1)
        .flags(vk::ImageCreateFlags::empty());

    let image = device.create_image(&info, None)?;
    let requirements = device.get_image_memory_requirements(image);
    let info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(get_memory_type_index(
            instance,
            data,
            properties,
            requirements,
        )?);

    let image_memory = device.allocate_memory(&info, None)?;
    device.bind_image_memory(image, image_memory, 0)?;

    Ok((image, image_memory))
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_texture_image(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    let image = File::open("textures/viking_room.png")?;
    let decoder = png::Decoder::new(image);
    let mut reader = decoder.read_info()?;

    let mut pixels = vec![0; reader.info().raw_bytes()];
    reader.next_frame(&mut pixels)?;

    let size = reader.info().raw_bytes() as u64;
    let (width, height) = reader.info().size();

    data.mip_levels = (width.max(height) as f32).log2().floor() as u32 + 1;

    unsafe {
        let (staging_buffer, staging_buffer_memory) = create_buffer(
            instance,
            device,
            data,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        let memory =
            device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;
        memcpy(pixels.as_ptr(), memory.cast(), pixels.len());
        device.unmap_memory(staging_buffer_memory);

        // create the vulkan image object to store the image data
        let (texture_image, texture_image_memory) = create_image(
            instance,
            device,
            data,
            width,
            height,
            data.mip_levels,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        data.texture_image = texture_image;
        data.texture_image_memory = texture_image_memory;

        transition_image_layout(
            device,
            data,
            data.texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            data.mip_levels,
        )?;

        copy_buffer_to_image(
            device,
            data,
            staging_buffer,
            data.texture_image,
            width,
            height,
        )?;

        transition_image_layout(
            device,
            data,
            data.texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            data.mip_levels,
        )?;

        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);

        generate_mipmaps(
            instance,
            device,
            data,
            data.texture_image,
            vk::Format::R8G8B8A8_SRGB,
            width,
            height,
            data.mip_levels,
        )?;
    }

    Ok(())
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn copy_buffer_to_image(
    device: &Device,
    data: &AppData,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
) -> Result<()> {
    unsafe {
        let command_buffer = begin_onetime_command(device, data)?;

        let subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1);

        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(subresource)
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        device.cmd_copy_buffer_to_image(
            command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );

        end_onetime_command(device, data, command_buffer)?;
    }

    Ok(())
}

pub unsafe fn transition_image_layout(
    device: &Device,
    data: &AppData,
    image: vk::Image,
    format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    mip_levels: u32,
) -> Result<()> {
    let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) =
        match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            ),
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ),
            _ => return Err(anyhow!("Unsupported Image Layout transition")),
        };

    let command_buffer = begin_onetime_command(device, data)?;

    let aspect_mask = if new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
        match format {
            vk::Format::D32_SFLOAT_S8_UINT | vk::Format::D24_UNORM_S8_UINT => {
                vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
            }
            _ => vk::ImageAspectFlags::DEPTH,
        }
    } else {
        vk::ImageAspectFlags::COLOR
    };

    let subresource = vk::ImageSubresourceRange::builder()
        .aspect_mask(aspect_mask)
        .base_mip_level(0)
        .level_count(mip_levels)
        .base_array_layer(0)
        .layer_count(1);

    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(subresource)
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask);

    device.cmd_pipeline_barrier(
        command_buffer,
        src_stage_mask,
        dst_stage_mask,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );

    end_onetime_command(device, data, command_buffer)?;

    Ok(())
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_image_view(
    device: &Device,
    image: vk::Image,
    format: vk::Format,
    aspects: vk::ImageAspectFlags,
    mip_levels: u32,
) -> Result<vk::ImageView> {
    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(aspects)
        .base_mip_level(0)
        .level_count(mip_levels)
        .base_array_layer(0)
        .layer_count(1);

    let info = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(vk::ImageViewType::_2D)
        .format(format)
        .subresource_range(subresource_range);

    Ok(unsafe { device.create_image_view(&info, None)? })
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn get_supported_format(
    instance: &Instance,
    data: &AppData,
    candidates: &[vk::Format],
    tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Result<vk::Format> {
    candidates
        .iter()
        .cloned()
        .find(|f| unsafe {
            let properties =
                instance.get_physical_device_format_properties(data.physical_device, *f);

            match tiling {
                vk::ImageTiling::LINEAR => properties.linear_tiling_features.contains(features),
                vk::ImageTiling::OPTIMAL => properties.optimal_tiling_features.contains(features),
                _ => false,
            }
        })
        .ok_or_else(|| anyhow!("Failed to find supported format"))
}

pub unsafe fn get_depth_format(instance: &Instance, data: &AppData) -> Result<vk::Format> {
    let candidates = &[
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];

    unsafe {
        get_supported_format(
            instance,
            data,
            candidates,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
    }
}

/// # Safety
/// This is a vulkan function and therfore unsafe
pub unsafe fn create_depth_objects(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    unsafe {
        let format = get_depth_format(instance, data)?;

        let (depth_image, depth_image_memory) = create_image(
            instance,
            device,
            data,
            data.swapchain_extent.width,
            data.swapchain_extent.height,
            1,
            format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        data.depth_image = depth_image;
        data.depth_image_memory = depth_image_memory;

        // image view

        data.depth_image_view = create_image_view(
            device,
            data.depth_image,
            format,
            vk::ImageAspectFlags::DEPTH,
            1,
        )?;

        transition_image_layout(
            device,
            data,
            data.depth_image,
            format,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            1,
        )?;

        // aspect mask and subresource creation

        Ok(())
    }
}

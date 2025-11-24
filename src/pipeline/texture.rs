use anyhow::{Ok, Result};
use vulkanalia::{
    Device,
    vk::{self, DeviceV1_0, HasBuilder},
};

use crate::{app::AppData, pipeline::image::create_image_view};

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_texture_sampler(device: &Device, data: &mut AppData) -> Result<()> {
    let info = vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(true)
        .max_anisotropy(16.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(data.mip_levels as f32);

    data.texture_sampler = device.create_sampler(&info, None)?;
    Ok(())
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_texture_image_view(device: &Device, data: &mut AppData) -> Result<()> {
    data.texture_image_view = unsafe {
        create_image_view(
            device,
            data.texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageAspectFlags::COLOR,
            data.mip_levels,
        )?
    };

    Ok(())
}

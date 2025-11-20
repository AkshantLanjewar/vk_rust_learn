use anyhow::{Ok, Result};
use vulkanalia::{
    Device,
    vk::{self, DeviceV1_0, HasBuilder},
};

use crate::app::AppData;

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
        .max_lod(0.0);

    data.texture_sampler = device.create_sampler(&info, None)?;
    Ok(())
}

pub unsafe fn create_texture_image_view(device: &Device, data: &mut AppData) -> Result<()> {
    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);

    let info = vk::ImageViewCreateInfo::builder()
        .image(data.texture_image)
        .view_type(vk::ImageViewType::_2D)
        .format(vk::Format::R8G8B8A8_SRGB)
        .subresource_range(subresource_range);

    data.texture_image_view = device.create_image_view(&info, None)?;
    Ok(())
}

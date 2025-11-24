use anyhow::{Result, anyhow};
use std::hash::Hasher;
use std::ptr::copy_nonoverlapping as memcpy;
use std::{hash::Hash, mem::size_of};

use cgmath::{vec2, vec3};
use vulkanalia::{
    Device, Instance,
    vk::{self, DeviceV1_0, Format, FormatFeatureFlags, Handle, HasBuilder, InstanceV1_0},
};

use crate::{
    app::AppData,
    pipeline::buffers::{begin_onetime_command, end_onetime_command},
};

pub type Vec2 = cgmath::Vector2<f32>;
pub type Vec3 = cgmath::Vector3<f32>;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: Vec3,
    pub tex_coord: Vec2,
}

impl Vertex {
    pub const fn new(pos: Vec3, color: Vec3, tex_coord: Vec2) -> Self {
        Self {
            pos,
            color,
            tex_coord,
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32)
            .build();

        let tex_coord = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset((size_of::<Vec3>() + size_of::<Vec3>()) as u32)
            .build();

        [pos, color, tex_coord]
    }
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos && self.color == other.color && self.tex_coord == other.tex_coord
    }
}

impl Eq for Vertex {}

impl Hash for Vertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pos[0].to_bits().hash(state);
        self.pos[1].to_bits().hash(state);
        self.pos[2].to_bits().hash(state);
        self.color[0].to_bits().hash(state);
        self.color[1].to_bits().hash(state);
        self.color[2].to_bits().hash(state);
        self.tex_coord[0].to_bits().hash(state);
        self.tex_coord[1].to_bits().hash(state);
    }
}

pub unsafe fn get_memory_type_index(
    instance: &Instance,
    data: &AppData,
    properties: vk::MemoryPropertyFlags,
    requirements: vk::MemoryRequirements,
) -> Result<u32> {
    let memory = instance.get_physical_device_memory_properties(data.physical_device);
    (0..memory.memory_type_count)
        .find(|i| {
            let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
            let memory_type = memory.memory_types[*i as usize];
            suitable && memory_type.property_flags.contains(properties)
        })
        .ok_or_else(|| anyhow!("Failed to find suitable memory type"))
}

pub unsafe fn create_buffer(
    instance: &Instance,
    device: &Device,
    data: &AppData,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = device.create_buffer(&buffer_info, None)?;

    let requirements = device.get_buffer_memory_requirements(buffer);
    let memory_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(get_memory_type_index(
            instance,
            data,
            properties,
            requirements,
        )?);

    let buffer_memory = device.allocate_memory(&memory_info, None)?;
    device.bind_buffer_memory(buffer, buffer_memory, 0)?;
    Ok((buffer, buffer_memory))
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn copy_buffer(
    device: &Device,
    data: &AppData,
    source: vk::Buffer,
    destination: vk::Buffer,
    size: vk::DeviceSize,
) -> Result<()> {
    let command_buffer = begin_onetime_command(device, data)?;

    let regions = vk::BufferCopy::builder().size(size);
    device.cmd_copy_buffer(command_buffer, source, destination, &[regions]);

    end_onetime_command(device, data, command_buffer)?;

    Ok(())
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_index_buffer(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    unsafe {
        let size = (size_of::<u32>() * data.indices.len()) as u64;

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
        memcpy(data.indices.as_ptr(), memory.cast(), data.indices.len());
        device.unmap_memory(staging_buffer_memory);

        let (index_buffer, index_buffer_memory) = create_buffer(
            instance,
            device,
            data,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        data.index_buffer = index_buffer;
        data.index_buffer_memory = index_buffer_memory;

        copy_buffer(device, data, staging_buffer, index_buffer, size)?;

        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }

    Ok(())
}

/// # Safety
/// This is a vulkan using function and thus is unsafe
pub unsafe fn create_vertex_buffer(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    unsafe {
        let size = (size_of::<Vertex>() * data.vertices.len()) as u64;
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
        memcpy(data.vertices.as_ptr(), memory.cast(), data.vertices.len());
        device.unmap_memory(staging_buffer_memory);

        let (vertex_buffer, vertex_buffer_memory) = create_buffer(
            instance,
            device,
            data,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        data.vertex_buffer = vertex_buffer;
        data.vertex_buffer_memory = vertex_buffer_memory;

        copy_buffer(device, data, staging_buffer, vertex_buffer, size)?;
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }

    Ok(())
}

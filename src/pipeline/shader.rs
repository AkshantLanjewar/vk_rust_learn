use anyhow::Result;
use vulkanalia::{
    Device,
    bytecode::Bytecode,
    vk::{self, DeviceV1_0, HasBuilder},
};

pub unsafe fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
    let bytecode = Bytecode::new(bytecode).unwrap();
    let info = vk::ShaderModuleCreateInfo::builder()
        .code(bytecode.code())
        .code_size(bytecode.code_size());

    Ok(device.create_shader_module(&info, None)?)
}

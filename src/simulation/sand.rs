use std::sync::Arc;

use vulkano::buffer::Subbuffer;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::device::{Device, Queue};

use crate::deploy_shader;
use crate::pass_structs::Material;
use vulkano::memory::allocator::{
    AllocationCreateInfo, GenericMemoryAllocator, MemoryAllocator, MemoryUsage,
};

mod sand_shader {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "src/shaders/sand_particle.glsl",
    }
}

pub fn tick(
    memory_allocator: &GenericMemoryAllocator<
        std::sync::Arc<vulkano::memory::allocator::FreeListAllocator>,
    >,
    device: &Arc<Device>,
    queue: &Arc<Queue>,
    world: &[Material],
) -> Vec<Material> {
    let buffer = upload_buffer(world.to_owned(), memory_allocator);
    let future = deploy_shader::deploy(
        sand_shader::load(device.clone()).expect("Failed to create compute shader."),
        device.clone(),
        queue.clone(),
        &buffer,
        [1, 1, 1],
    );
    future.wait(None).unwrap();
    let binding = buffer.read().unwrap();
    let mut new: Vec<Material> = Vec::new();
    for (key, val) in binding.iter().enumerate() {
        if key <= 1 {
            // let out = val.pos;
            println!("{val:?}");
        }
        new.push(val.clone());
    }
    new
}

pub fn upload_buffer(
    data: Vec<Material>,
    memory_allocator: &(impl MemoryAllocator + ?Sized),
) -> Subbuffer<[Material]> {
    Buffer::from_iter(
        memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            usage: MemoryUsage::Upload,
            ..Default::default()
        },
        data,
    )
    .expect("failed to create buffer")
}

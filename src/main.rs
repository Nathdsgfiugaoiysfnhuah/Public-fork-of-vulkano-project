use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage};

use vulkano::memory::allocator::{
    AllocationCreateInfo, GenericMemoryAllocator, MemoryUsage, StandardMemoryAllocator,
};
use vulkano::sync::{self};

use crate::pass_structs::Material;

mod deploy_shader;
mod gpu_constructor;
mod pass_structs;
mod simulation;
mod window;

#[derive(BufferContents)]
#[repr(C)]
struct TestStruct {
    first: i32,
    second: i32,
    res: i32,
}

// device, queues,

fn main() {
    let mut world: Vec<Material> = Vec::new();
    for i in 1..64 {
        let i_f = i as f32;
        world.push(Material {
            id: i,
            colour: [i_f / 100f32, i_f / 100f32, i_f / 100f32],
            pos: [i_f, 100f32],
            ..Default::default()
        })
    }

    let (library, _physical_device, _queue_family_index, _instance, device, mut queues) =
        gpu_constructor::construct_gpu();

    // -=-=-=-=-=

    let queue = queues.next().unwrap();

    // let command_buffer_allocator = StandardCommandBufferAllocator::new(
    //     device.clone(),
    //     StandardCommandBufferAllocatorCreateInfo::default(),
    // );
    let memory_allocator: GenericMemoryAllocator<
        std::sync::Arc<vulkano::memory::allocator::FreeListAllocator>,
    > = StandardMemoryAllocator::new_default(device.clone());

    // let data: TestStruct = TestStruct {
    //     first: 5,
    //     second: 7,
    //     res: 10,
    // };
    let mut data = Vec::new();
    for a in 1..=20 {
        for b in 1..=20 {
            data.push(TestStruct {
                first: a,
                second: b,
                res: 0,
            });
        }
        // data.push(a);
    }

    let data2 = 0..64; //staging, gpu 1, gpu 2, download
    let buffer = Buffer::from_iter(
        &memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            usage: MemoryUsage::Upload,
            ..Default::default()
        },
        data2,
    )
    .expect("failed to create buffer");
    println!("buffer (pogger)");

    mod cs {
        vulkano_shaders::shader! {
            ty: "compute",
            path: "src/shaders/test/test.frag",
        }
    }

    let shader = cs::load(device.clone()).expect("failed to create shader module");

    let future = deploy_shader::deploy(shader, device.clone(), queue.clone(), &buffer, [1, 1, 1]);

    future.wait(None).unwrap();
    // let binding = buffer.read().unwrap();
    // for val in binding.iter() {
    //     println!("{val}");
    // }

    window::make_window(library, memory_allocator, device, queue, world);
    //main.rs is done now as window now has control
}

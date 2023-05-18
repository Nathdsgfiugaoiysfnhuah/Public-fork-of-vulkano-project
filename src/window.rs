use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};

use vulkano::descriptor_set::allocator::{DescriptorSetAllocator, StandardDescriptorSetAllocator};
use vulkano::descriptor_set::{self, PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};

use vulkano::memory::allocator::{
    AllocationCreateInfo, GenericMemoryAllocator, MemoryUsage, StandardMemoryAllocator,
};

use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::Pipeline;
use vulkano::swapchain::{
    acquire_next_image, Swapchain, SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::swapchain::{AcquireError, SwapchainCreateInfo};
use vulkano::VulkanLibrary;

use vulkano::sync::future::FenceSignalFuture;
use vulkano::sync::{self, FlushError, GpuFuture};
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use crate::pass_structs::{Material, WindowInitialized};
use crate::simulation::sand;

mod fps;
mod init;
mod utils;

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path:"src/shaders/test/test_vert.vert"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path:"src/shaders/test/test_frag.frag"
    }
}

const FPS_DISPLAY: bool = true;

pub fn make_window(
    library: Arc<VulkanLibrary>,
    compute_memory_allocator: GenericMemoryAllocator<
        std::sync::Arc<vulkano::memory::allocator::FreeListAllocator>,
    >,
    compute_device: Arc<Device>,
    compute_queue: Arc<Queue>,
    mut world: Vec<Material>,
) {
    let WindowInitialized {
        physical_device: render_physical_device,
        surface,
        device: render_device,
        window,
        mut window_size,
        event_loop,
        queue: render_queue,
    } = init::initialize_window(&library);

    let (mut swapchain, mut images) = utils::get_swapchain(
        &render_physical_device,
        &render_device,
        &window,
        surface.clone(),
    );
    let render_pass = utils::get_render_pass(render_device.clone(), swapchain.clone());
    let frame_buffers = utils::get_framebuffers(&images, render_pass.clone());

    let render_memory_allocator = StandardMemoryAllocator::new_default(render_device.clone());

    let vertex1 = utils::CPUVertex {
        position: [-1.0, -1.0],
    };
    let vertex2 = utils::CPUVertex {
        position: [3.0, -1.0], // 3 because -1 -> 1 => width = 2, 1 + 2 = 3
    };
    let vertex3 = utils::CPUVertex {
        position: [-1.0, 3.0],
    };
    // let vertex4 = utils::CPUVertex {
    //     position: [0.5, 0.5],
    // }; Clipping makes this useless, see https://www.saschawillems.de/blog/2016/08/13/vulkan-tutorial-on-rendering-a-fullscreen-quad-without-buffers/
    let vertex_buffer = Buffer::from_iter(
        &render_memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            usage: MemoryUsage::Upload,
            ..Default::default()
        },
        vec![vertex1, vertex2, vertex3],
    )
    .unwrap();

    let vs = vs::load(render_device.clone()).expect("failed to create shader module");
    let fs = fs::load(render_device.clone()).expect("failed to create shader module");

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: window_size.into(),
        depth_range: 0.0..1.0,
    };

    let mut recreate_swapchain = false;
    let frames_in_flight = images.len();
    let mut fences: Vec<Option<Arc<FenceSignalFuture<_>>>> = vec![None; frames_in_flight];
    let mut previous_fence_i = 0;

    let render_pipeline = utils::get_pipeline(
        render_device.clone(),
        vs.clone(),
        fs.clone(),
        render_pass.clone(),
        viewport.clone(),
    );

    let command_buffers = utils::get_command_buffers(
        &render_device,
        &render_queue,
        &render_pipeline,
        &frame_buffers,
        &vertex_buffer,
    );

    //fps
    let mut frames = [0f64; 60];
    let mut cur_frame = 0;
    let mut time = 0f64;
    // let frames_r = &mut frames; winit static garbo or smth, idk why this does not work.
    // let cur_frame_r = &mut cur_frame;
    // let time_r = &mut time;
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event:
                WindowEvent::CursorMoved {
                    device_id: _,
                    position: _,
                    ..
                },
            ..
        } => {
            // println!("{position:?}");
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            recreate_swapchain = true;
        }
        Event::RedrawEventsCleared => {
            if recreate_swapchain {
                // println!("recreating swapchain (slow)");
                recreate_swapchain = false;

                let new_dimensions = window.inner_size();
                window_size = new_dimensions;

                let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                    image_extent: new_dimensions.into(), // here, "image_extend" will correspond to the window dimensions
                    ..swapchain.create_info()
                }) {
                    Ok(r) => r,
                    // This error tends to happen when the user is manually resizing the window.
                    // Simply restarting the loop is the easiest way to fix this issue.
                    Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                    Err(e) => panic!("failed to recreate swapchain: {e}"),
                };
                swapchain = new_swapchain;
            }

            let (image_i, suboptimal, acquire_future) =
                match acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("failed to acquire next image: {e}"),
                };
            if suboptimal {
                recreate_swapchain = true;
            }

            // wait for the fence related to this image to finish (normally this would be the oldest fence)
            if let Some(image_fence) = &fences[image_i as usize] {
                image_fence.wait(None).unwrap();
            }

            let previous_future = match fences[previous_fence_i as usize].clone() {
                // Create a NowFuture
                None => {
                    let mut now = sync::now(render_device.clone());
                    now.cleanup_finished();

                    now.boxed()
                }
                // Use the existing FenceSignalFuture
                Some(fence) => fence.boxed(),
            };

            let future = previous_future
                .join(acquire_future)
                .then_execute(
                    render_queue.clone(),
                    command_buffers[image_i as usize].clone(),
                )
                .unwrap()
                .then_swapchain_present(
                    render_queue.clone(),
                    SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_i),
                )
                .then_signal_fence_and_flush();

            fences[image_i as usize] = match future {
                Ok(value) => Some(Arc::new(value)),
                Err(FlushError::OutOfDate) => {
                    recreate_swapchain = true;
                    None
                }
                Err(e) => {
                    println!("failed to flush future: {e}");
                    None
                }
            };
            previous_fence_i = image_i;
            if FPS_DISPLAY {
                fps::do_fps(&mut frames, &mut cur_frame, &mut time);
            }
            world = sand::tick(
                &compute_memory_allocator,
                &compute_device.clone(),
                &compute_queue.clone(),
                &world,
            );
        }
        _ => (),
    });
}

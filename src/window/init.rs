use std::sync::Arc;

use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::CommandBufferExecFuture;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::Queue;
use vulkano::device::{
    physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    QueueFlags,
};

use vulkano::instance::{Instance, InstanceCreateInfo};

use vulkano::VulkanLibrary;

use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::RenderPass;
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{PresentFuture, Surface, SwapchainAcquireFuture};
use vulkano::sync::future::{FenceSignalFuture, JoinFuture};
use vulkano::sync::GpuFuture;
use vulkano_win::VkSurfaceBuild;

use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::pass_structs::WindowInitialized;

use super::utils::{self, CPUVertex};

type FenceExpanded = Option<
    Arc<
        FenceSignalFuture<
            PresentFuture<
                CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>,
            >,
        >,
    >,
>;

pub fn initialize_window(library: &Arc<VulkanLibrary>) -> WindowInitialized {
    let required_extensions = vulkano_win::required_extensions(library);
    let instance = Instance::new(
        library.clone(),
        InstanceCreateInfo {
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )
    .expect("failed to create instance");

    let event_loop = EventLoop::new();

    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let window = surface
        .object()
        .unwrap()
        .clone()
        .downcast::<Window>()
        .unwrap();

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };

    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()
        .expect("failed to get devices")
        .filter(|p| p.supported_extensions().contains(&device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("no device available");

    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_extensions: device_extensions, // new
            ..Default::default()
        },
    ).expect("failed to create window device? how could the buffer succeed and this fail, gpu isn't plugged in?");

    let queue = queues.next().unwrap();

    let caps = physical_device
        .surface_capabilities(&surface, Default::default())
        .expect("failed to get surface capabilities");

    let window_size = window.inner_size();
    let _composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
    let _image_format = Some(
        physical_device
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0,
    );
    WindowInitialized {
        physical_device, // cool rust feature you don't need field names if its the same
        surface,
        device,
        window,
        window_size,
        event_loop,
        queue,
    }
}

pub fn initialize_window_from_preexisting(
    physical_device: Arc<PhysicalDevice>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    library: &Arc<VulkanLibrary>,
) -> WindowInitialized {
    let required_extensions = vulkano_win::required_extensions(library);
    let instance = Instance::new(
        library.clone(),
        InstanceCreateInfo {
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )
    .expect("failed to make instance");
    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();
    let window = surface
        .object()
        .unwrap()
        .clone()
        .downcast::<Window>()
        .unwrap();
    let (swapchain, images) =
        utils::get_swapchain(&physical_device, &device, &window, surface.clone());
    let window_size = window.inner_size();
    WindowInitialized {
        physical_device, // cool rust feature you don't need field names if its the same
        surface,
        device,
        window,
        window_size,
        event_loop,
        queue,
    }
}

pub fn initialize_swapchain_screen<T>(
    render_physical_device: Arc<PhysicalDevice>,
    render_device: Arc<Device>,
    window: Arc<Window>,
    surface: Arc<Surface>,
    window_size: PhysicalSize<u32>,
    render_queue: Arc<Queue>,
    buffer: &Subbuffer<[T]>,
) -> (
    std::sync::Arc<vulkano::swapchain::Swapchain>,
    bool,
    std::vec::Vec<std::sync::Arc<vulkano::command_buffer::PrimaryAutoCommandBuffer>>,
    Viewport,
    Arc<RenderPass>,
    Arc<ShaderModule>,
    Arc<ShaderModule>,
    Subbuffer<[CPUVertex]>,
    Vec<FenceExpanded>,
    u32,
) {
    let (swapchain, images) =
        utils::get_swapchain(&render_physical_device, &render_device, &window, surface);
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

    let vs_loaded = vs::load(render_device.clone()).expect("failed to create shader module");
    let fs_loaded = fs::load(render_device.clone()).expect("failed to create shader module");

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: window_size.into(),
        depth_range: 0.0..1.0,
    };

    let recreate_swapchain = false;
    let frames_in_flight = images.len();
    let fences: Vec<FenceExpanded> = vec![None; frames_in_flight];
    let previous_fence_i = 0;

    let render_pipeline = utils::get_pipeline(
        render_device.clone(),
        vs_loaded.clone(),
        fs_loaded.clone(),
        render_pass.clone(),
        viewport.clone(),
    );
    let push_constants = fs::PushType {
        dims: [window_size.width as f32, window_size.height as f32],
    };
    let command_buffers = utils::get_command_buffers(
        &render_device,
        &render_queue,
        &render_pipeline,
        &frame_buffers,
        &vertex_buffer,
        push_constants,
        buffer,
    );

    (
        swapchain,
        recreate_swapchain,
        command_buffers,
        viewport,
        render_pass,
        vs_loaded,
        fs_loaded,
        vertex_buffer,
        fences,
        previous_fence_i,
    )
}

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path:"src/shaders/test/test_vert.vert"
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path:"src/shaders/test/test_frag.frag"
    }
}

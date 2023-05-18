use std::sync::Arc;



use vulkano::device::{
    physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    QueueFlags,
};

use vulkano::instance::{Instance, InstanceCreateInfo};

use vulkano::VulkanLibrary;



use vulkano_win::VkSurfaceBuild;


use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::pass_structs::WindowInitialized;

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

use std::sync::Arc;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::VulkanLibrary;

pub fn construct_gpu() -> (
    Arc<VulkanLibrary>,
    Arc<PhysicalDevice>,
	u32,
    Arc<Instance>,
    Arc<Device>,
    impl ExactSizeIterator + Iterator<Item = Arc<Queue>>,
) {
    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let instance = Instance::new(library.clone(), InstanceCreateInfo::default())
        .expect("failed to create instance");
    let physical_device = instance
        .enumerate_physical_devices()
        .expect("could not enumerate devices")
        .next()
        .expect("no devices available");
    let name = &physical_device.properties().device_name;
    println!("{name:?}");
    for family in physical_device.queue_family_properties() {
        println!(
            "Found a queue family with {:?} queue(s)",
            family.queue_count
        );
    }

    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .enumerate()
        .position(|(_queue_family_index, queue_family_properties)| {
            queue_family_properties
                .queue_flags
                .contains(QueueFlags::GRAPHICS)
        })
        .expect("couldn't find a graphical queue family") as u32;

    let result = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            // here we pass the desired queue family to use by index
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_extensions: DeviceExtensions {
                khr_storage_buffer_storage_class: true,
                ..DeviceExtensions::empty()
            },
            ..Default::default()
        },
    )
    .expect("failed to create device");
    println!("Device acquired");

    (
        library,
        physical_device,
        queue_family_index,
        instance,
        result.0,
        result.1,
    )
}

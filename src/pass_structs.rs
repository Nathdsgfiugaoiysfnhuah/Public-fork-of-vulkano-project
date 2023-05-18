use std::sync::Arc;

use vulkano::buffer::BufferContents;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::Device;
use vulkano::device::Queue;

use vulkano::swapchain::Surface;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::Window;
pub struct WindowInitialized {
    pub physical_device: Arc<PhysicalDevice>,
    pub surface: Arc<Surface>,
    pub device: Arc<Device>,
    pub window: Arc<Window>,
    pub window_size: PhysicalSize<u32>,
    pub event_loop: EventLoop<()>,
    pub queue: Arc<Queue>,
}

#[repr(C)]
#[derive(BufferContents, Clone, Debug)]
pub struct Material {
    // MUST BE KEPT IN SYNC WITH GLSL VERSION
    pub id: u32,
    pub _pad: u64,        // misalignment dirty fix
    pub colour: [f32; 3], // made using id normally except it can also change
    pub _pad2: u16,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub mass: f32, // made using id
    pub _pad3: u16,
    pub target: [f32; 2], // ^ for statics
    pub force: f32,       // ^ how much force towards target is left?
    pub stable: f32,      // ^ how much displacement is the maximum before force weakens?
    pub tags: u32,        // ^ is a bit mask of tags in the future maybe, currently useless
    pub gas: u32,         // ^ 0 is normal gravity, 1 is antigravity, other values will do smth idk
}

impl Default for Material {
    fn default() -> Material {
        Material {
            id: 0,
            _pad: 64,
            colour: [1f32, 0f32, 1f32],
            _pad2: 16,
            pos: [0f32, 0f32],
            vel: [0f32, 0f32],
            mass: 1f32,
            _pad3: 16,
            target: [0f32, 0f32],
            force: 0f32,
            stable: 0f32,
            tags: 0,
            gas: 0,
        }
    }
}
// pub struct GpuConstructed {
// 	pub vulkan_library: Arc<VulkanLibrary>,
//     pub physical_device: Arc<PhysicalDevice>,
// 	pub queue_family_index: u32,
//     pub instance: Arc<Instance>,
//     pub device: Arc<Device>,
//     pub queues: dyn Iterator<Item = Arc<Queue>>,
// }

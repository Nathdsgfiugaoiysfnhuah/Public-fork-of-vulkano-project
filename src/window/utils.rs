use crate::window::Arc;
use vulkano::buffer::{BufferContents, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
    SubpassContents,
};

use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{Device, Queue};
use vulkano::image::ImageUsage;
use vulkano::image::{view::ImageView, SwapchainImage};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{
    PresentMode, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
};
use winit::window::Window;

use super::init;

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct CPUVertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
}

pub fn get_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.image_format(), // set the format the same as the swapchain
                samples: 1,
            },
        },
        pass: {
            color: [color],
            depth_stencil: {},
        },
    )
    .unwrap()
}

pub fn get_framebuffers(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}

pub fn get_pipeline(
    device: Arc<Device>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport,
) -> Arc<GraphicsPipeline> {
    GraphicsPipeline::start()
        .vertex_input_state(CPUVertex::per_vertex())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass, 0).unwrap())
        .build(device)
        .unwrap()
}

pub fn get_command_buffers<T>(
    device: &Arc<Device>,
    queue: &Arc<Queue>,
    pipeline: &Arc<GraphicsPipeline>,
    frame_buffers: &[Arc<Framebuffer>],
    vertex_buffer: &Subbuffer<[CPUVertex]>,
    push_constants: init::fs::PushType,
    buffer: &Subbuffer<[T]>,
) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    let command_buffer_allocator =
        StandardCommandBufferAllocator::new(device.clone(), Default::default());
    frame_buffers
        .iter()
        .map(|frame_buffer| {
            build_render_pass(
                frame_buffer,
                queue,
                pipeline,
                vertex_buffer,
                &command_buffer_allocator,
                push_constants,
                buffer,
                device,
            )
        })
        .collect()
}

fn build_render_pass<T>(
    frame_buffer: &Arc<Framebuffer>,
    queue: &Arc<Queue>,
    pipeline: &Arc<GraphicsPipeline>,
    vertex_buffer: &Subbuffer<[CPUVertex]>,
    command_buffer_allocator: &StandardCommandBufferAllocator,
    push_constants: init::fs::PushType,
    buffer: &Subbuffer<[T]>,
    device: &Arc<Device>,
) -> Arc<PrimaryAutoCommandBuffer> {
    let mut builder = AutoCommandBufferBuilder::primary(
        command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::MultipleSubmit,
    )
    .unwrap();

    let layout = pipeline.layout();
    let descriptor_set_layouts = layout.set_layouts();
    println!("{descriptor_set_layouts:?}");
    let descriptor_set_layout = descriptor_set_layouts.get(0).unwrap();

    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());

    let descriptor_set = match PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::buffer(0, buffer.clone())], // 0 is the binding
    ) {
        Ok(res) => res,
        Err(e) => panic!("Error with {e:?}"),
    };

    builder
        .begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![Some([1.0, 0.0, 1.0, 1.0].into())],
                ..RenderPassBeginInfo::framebuffer(frame_buffer.clone())
            },
            SubpassContents::Inline,
        )
        .unwrap()
        .bind_pipeline_graphics(pipeline.clone())
        .bind_vertex_buffers(0, vertex_buffer.clone())
        .push_constants(layout.clone(), 0, push_constants)
        .draw(vertex_buffer.len() as u32, 1, 0, 0)
        .unwrap()
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            layout.clone(),
            0,
            descriptor_set,
        )
        .end_render_pass()
        .unwrap();

    Arc::new(builder.build().unwrap())
}

pub fn get_swapchain(
    render_physical_device: &Arc<PhysicalDevice>,
    render_device: &Arc<Device>,
    window: &std::sync::Arc<winit::window::Window>,
    surface: Arc<Surface>,
) -> (Arc<Swapchain>, Vec<Arc<SwapchainImage>>) {
    let (swapchain, images) = {
        let caps = render_physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let dimensions = window.inner_size();
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = Some(
            render_physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        Swapchain::new(
            render_device.clone(),
            surface,
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count,
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                present_mode: PresentMode::Mailbox, //TODO add support for GPUs which don't have mailbox support, apparently immediate is second best.
                ..Default::default()
            },
        )
        .unwrap()
    };
    (swapchain, images)
}

pub fn recreate_swapchain<T>(
    window: &Window,
    render_pass: &Arc<RenderPass>,
    swapchain: &mut Arc<Swapchain>,
    viewport: &mut Viewport,
    render_device: &Arc<Device>,
    render_queue: &Arc<Queue>,
    vertex_buffer: &Subbuffer<[CPUVertex]>,
    command_buffers: &mut Vec<Arc<PrimaryAutoCommandBuffer>>,
    vs: &Arc<ShaderModule>,
    fs: &Arc<ShaderModule>,
    buffer: &Subbuffer<[T]>,
    push_constants: init::fs::PushType,
) {
    let new_dimensions = window.inner_size();

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
    *swapchain = new_swapchain;
    let frame_buffers = get_framebuffers(&new_images, render_pass.clone());
    viewport.dimensions = new_dimensions.into();
    let new_pipeline = get_pipeline(
        render_device.clone(),
        vs.clone(),
        fs.clone(),
        render_pass.clone(),
        viewport.clone(),
    );
    *command_buffers = get_command_buffers(
        render_device,
        render_queue,
        &new_pipeline,
        &frame_buffers,
        vertex_buffer,
        push_constants,
        buffer,
    );
}

use crate::sync::future::FenceSignalFuture;
use crate::sync::future::NowFuture;
use std::sync::Arc;
use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::CommandBufferExecFuture;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};
use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint};
use vulkano::shader::ShaderModule;
use vulkano::sync::{self, GpuFuture};

pub fn deploy(
    device: Arc<Device>,
    queue: Arc<Queue>,
    command: Arc<PrimaryAutoCommandBuffer>,
) -> FenceSignalFuture<CommandBufferExecFuture<NowFuture>> {

    sync::now(device)
        .then_execute(queue, command)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap()
}

pub fn get_deploy_command<T>(
    shader: &Arc<ShaderModule>,
    device: &Arc<Device>,
    queue: &Arc<Queue>,
    buffer: &Subbuffer<[T]>,
    work_group_counts: [u32; 3],
) -> vulkano::command_buffer::PrimaryAutoCommandBuffer {
    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        shader.entry_point("main").unwrap(),
        &(),
        None,
        |_| {},
    )
    .expect("failed to create compute pipeline");

    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
    let pipeline_layout = compute_pipeline.layout();
    let descriptor_set_layouts = pipeline_layout.set_layouts();
    let descriptor_set_layout_index = 0;
    let descriptor_set_layout = descriptor_set_layouts
        .get(descriptor_set_layout_index)
        .unwrap();

    let descriptor_set = match PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::buffer(0, buffer.clone())], // 0 is the binding
    ) {
        Ok(res) => res,
        Err(e) => panic!("Error with {e:?}"),
    };

    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );
    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::MultipleSubmit,
    )
    .unwrap();

    command_buffer_builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            compute_pipeline.layout().clone(),
            descriptor_set_layout_index as u32,
            descriptor_set,
        )
        .dispatch(work_group_counts)
        .unwrap();

    command_buffer_builder.build().unwrap()
}

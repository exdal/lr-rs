use ash::vk;
use gpu_allocator::vulkan;

/////////////////////////////////
// IMAGES
pub struct Image {
    pub usage: vk::ImageUsageFlags,
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub slices: u32,
    pub levels: u32,

    pub allocation: Option<vulkan::Allocation>,
    pub handle: vk::Image,
}
define_from!(Image, vk::Image);

pub struct ImageView {
    pub format: vk::Format,
    pub subresource_range: vk::ImageSubresourceRange,

    pub handle: vk::ImageView,
}
define_from!(ImageView, vk::ImageView);

pub struct Sampler {
    pub handle: vk::Sampler,
}
define_from!(Sampler, vk::Sampler);

/////////////////////////////////
// BUFFERS
pub struct Buffer {
    pub data_size: u64,
    pub device_address: u64,

    pub allocation: vulkan::Allocation,
    pub handle: vk::Buffer,
}
define_from!(Buffer, vk::Buffer);

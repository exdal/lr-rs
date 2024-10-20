use std::{marker::PhantomData, mem::MaybeUninit};

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
type BufferID = u32;
pub struct Buffer {
    pub data_size: u64,
    pub device_address: u64,

    pub allocation: vulkan::Allocation,
    pub handle: vk::Buffer,
}
define_from!(Buffer, vk::Buffer);

/////////////////////////////////
// DESCRIPTORS
#[derive(Default)]
pub struct DescriptorPool(pub vk::DescriptorPool);
define_from_tupl!(DescriptorPool, vk::DescriptorPool, 0);
#[derive(Default)]
pub struct DescriptorSetLayout(pub vk::DescriptorSetLayout);
define_from_tupl!(DescriptorSetLayout, vk::DescriptorSetLayout, 0);
#[derive(Default)]
pub struct DescriptorSet(pub vk::DescriptorSet);
define_from_tupl!(DescriptorSet, vk::DescriptorSet, 0);

/////////////////////////////////
// RESOURCE POOL
const MAX_RESOURCE_COUNT: u32 = 1 << 19;
const PAGE_BITS: u32 = 9;
const PAGE_SIZE: u32 = 1 << PAGE_BITS;
const PAGE_MASK: u32 = PAGE_SIZE - 1;
const PAGE_COUNT: u32 = MAX_RESOURCE_COUNT / PAGE_SIZE;

type Page<T> = [MaybeUninit<T>; PAGE_SIZE as usize];

pub struct ResourcePool<ResourceT, ResourceID>
where
    ResourceID: Into<u32>,
{
    pub pages: [Option<Box<Page<ResourceT>>>; PAGE_COUNT as usize],
    pub free_indices: Vec<u32>,
    pub latest_index: u32,
    _rust: PhantomData<ResourceID>, // ???
}

impl<ResourceT, ResourceID> ResourcePool<ResourceT, ResourceID>
where
    ResourceID: Into<u32>,
{
    fn new() -> Self {
        Self {
            latest_index: 0,
            free_indices: Vec::new(),
            pages: [const { None }; PAGE_COUNT as usize],
            _rust: PhantomData,
        }
    }

    fn create(&mut self, args: impl FnOnce() -> ResourceT) -> Option<(&ResourceT, ResourceID)>
    where
        ResourceID: From<u32>,
    {
        let index: u32;
        if self.free_indices.is_empty() {
            self.latest_index += 1;
            index = self.latest_index;
            if index >= MAX_RESOURCE_COUNT {
                return None;
            }
        } else {
            index = *self.free_indices.last().unwrap();
            self.free_indices.pop();
        }

        let page_id = index >> PAGE_BITS;
        let page_offset = index & PAGE_MASK;
        if page_id >= PAGE_COUNT {
            return None;
        }

        if self.pages[page_id as usize].is_none() {
            self.pages[page_id as usize] = Some(Box::new(
                [const { MaybeUninit::uninit() }; PAGE_SIZE as usize],
            ));
        }

        let page = self.pages[page_id as usize].as_mut().unwrap();
        let resource: &mut ResourceT = unsafe {
            page[page_offset as usize].as_mut_ptr().write(args());
            &mut *page[page_offset as usize].as_mut_ptr()
        };

        Some((resource, ResourceID::from(index)))
    }
}

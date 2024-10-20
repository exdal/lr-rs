use ash::{khr, vk};
use gpu_allocator::vulkan;
use winit::window;

use super::{
    Buffer, CommandAllocator, CommandList, CommandQueue, CommandType, DescriptorPool,
    DescriptorSet, DescriptorSetLayout, Image, ImageView, PhysicalDevice, Sampler, Semaphore,
    SwapChain,
};

#[repr(u32)]
enum Descriptor {
    Samplers(vk::DescriptorType, u32) = 0,
    Images(vk::DescriptorType, u32) = 1,
    StorageImages(vk::DescriptorType, u32) = 2,
    StorageBuffer(vk::DescriptorType, u32) = 3,
    BufferDeviceaddress(vk::DescriptorType, u32) = 4,
}

pub struct Device {
    pub physical_device: PhysicalDevice,
    pub swapchain_loader: khr::swapchain::Device,

    pub queues: [CommandQueue; 3],
    pub allocator: vulkan::Allocator,
    pub handle: ash::Device,
    pub frame_sema: Semaphore,
    pub frame_count: u32,

    // BINDLESS DESCRIPTOR SET //
    pub descriptor_pool: DescriptorPool,
    pub descriptor_set_layout: DescriptorSetLayout,
    pub descriptor_set: DescriptorSet,
}

impl Device {
    pub fn new(frame_count: u32) -> Result<Self, vk::Result> {
        let physical_device = PhysicalDevice::new()?;
        let handle = physical_device.create_device()?;
        let swapchain_loader = khr::swapchain::Device::new(&physical_device.instance, &handle);
        let queues = [CommandQueue::default(); 3];

        let allocator = vulkan::Allocator::new(&vulkan::AllocatorCreateDesc {
            instance: physical_device.instance.clone(),
            device: handle.clone(),
            physical_device: physical_device.handle,
            debug_settings: Default::default(),
            buffer_device_address: true,
            allocation_sizes: Default::default(),
        })
        .expect("Failed to create allocator");

        println!(
            "Initialized Vulkan for Physicial Device @ 0 ({}-{:?})",
            physical_device.properties.api_version,
            &physical_device.properties.device_name_as_c_str().unwrap()
        );

        let mut result = Self {
            physical_device,
            swapchain_loader,
            queues,
            allocator,
            handle,
            frame_sema: Default::default(),
            frame_count,
            descriptor_pool: DescriptorPool::default(),
            descriptor_set_layout: DescriptorSetLayout::default(),
            descriptor_set: DescriptorSet::default(),
        };

        // Preparation
        result.frame_sema = result.create_timeline_semaphore()?;
        let native_queues = unsafe {
            [
                result.handle.get_device_queue(
                    result.physical_device.queue_type_indices[CommandType::Graphics as usize]
                        as u32,
                    0,
                ),
                result.handle.get_device_queue(
                    result.physical_device.queue_type_indices[CommandType::Compute as usize] as u32,
                    0,
                ),
                result.handle.get_device_queue(
                    result.physical_device.queue_type_indices[CommandType::Transfer as usize]
                        as u32,
                    0,
                ),
            ]
        };

        (0..3).for_each(|i| {
            result.queues[i] = CommandQueue {
                family_index: result.physical_device.queue_type_indices[i] as u32,
                semaphore: result.create_timeline_semaphore().unwrap(),
                handle: native_queues[i],
            }
        });

        // TODO: Replace this amount with ResourcePool size in the future
        let fixed_descriptor_count = 1024_u32;

        let descriptor_set_layout_infos = [
            Descriptor::Samplers(vk::DescriptorType::SAMPLER, fixed_descriptor_count),
            Descriptor::Images(vk::DescriptorType::SAMPLED_IMAGE, fixed_descriptor_count),
            Descriptor::StorageImages(vk::DescriptorType::STORAGE_IMAGE, fixed_descriptor_count),
            Descriptor::StorageBuffer(vk::DescriptorType::STORAGE_BUFFER, fixed_descriptor_count),
            Descriptor::BufferDeviceaddress(vk::DescriptorType::STORAGE_BUFFER, 1_u32),
        ];

        let mut descriptor_pool_sizes = Vec::new();
        let mut descriptor_bindings = Vec::new();
        let mut descriptor_binding_flags = Vec::new();

        descriptor_set_layout_infos
            .iter()
            .map(|descriptor| match descriptor {
                Descriptor::Samplers(descriptor_type, descriptor_count)
                | Descriptor::Images(descriptor_type, descriptor_count)
                | Descriptor::StorageImages(descriptor_type, descriptor_count)
                | Descriptor::StorageBuffer(descriptor_type, descriptor_count)
                | Descriptor::BufferDeviceaddress(descriptor_type, descriptor_count) => {
                    (*descriptor_type, *descriptor_count)
                }
            })
            .enumerate()
            .for_each(|(binding, (descriptor_type, descriptor_count))| {
                let pool_size = vk::DescriptorPoolSize::default()
                    .ty(descriptor_type)
                    .descriptor_count(descriptor_count);
                let binding_info = vk::DescriptorSetLayoutBinding::default()
                    .descriptor_type(descriptor_type)
                    .binding(binding as u32);
                let binding_flags = vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
                    | vk::DescriptorBindingFlags::PARTIALLY_BOUND;

                descriptor_pool_sizes.push(pool_size);
                descriptor_bindings.push(binding_info);
                descriptor_binding_flags.push(binding_flags);
            });

        let mut descriptor_binding_flag_info =
            vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
                .binding_flags(&descriptor_binding_flags);
        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .bindings(&descriptor_bindings)
            .push_next(&mut descriptor_binding_flag_info);
        result.descriptor_set_layout.0 = unsafe {
            result
                .handle
                .create_descriptor_set_layout(&descriptor_set_layout_info, None)
                .expect("Failed to create bindless descriptor set")
        };

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .pool_sizes(&descriptor_pool_sizes)
            .max_sets(1);
        result.descriptor_pool.0 = unsafe {
            result
                .handle
                .create_descriptor_pool(&descriptor_pool_info, None)
                .expect("Failed to create bindless descriptor pool")
        };

        let descriptor_layouts = [result.descriptor_set_layout.0];
        let descriptor_set_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(result.descriptor_pool.0)
            .set_layouts(&descriptor_layouts);
        result.descriptor_set.0 = unsafe {
            result
                .handle
                .allocate_descriptor_sets(&descriptor_set_info)
                .expect("Failed to allocate bindless descriptor set")[0]
        };

        Ok(result)
    }

    pub fn queue_at(&self, command_type: CommandType) -> &CommandQueue {
        &self.queues[command_type as usize]
    }

    pub fn create_binary_semaphore(&self) -> Result<Semaphore, vk::Result> {
        let mut semaphore_type_info =
            vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::BINARY);
        let create_info = vk::SemaphoreCreateInfo::default().push_next(&mut semaphore_type_info);
        let semaphore = unsafe { self.handle.create_semaphore(&create_info, None)? };

        Ok(Semaphore {
            counter: 0,
            handle: semaphore,
        })
    }

    pub fn create_timeline_semaphore(&self) -> Result<Semaphore, vk::Result> {
        let mut semaphore_type_info = vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(0);
        let create_info = vk::SemaphoreCreateInfo::default().push_next(&mut semaphore_type_info);
        let semaphore = unsafe { self.handle.create_semaphore(&create_info, None)? };

        Ok(Semaphore {
            counter: 0,
            handle: semaphore,
        })
    }

    pub fn wait_for_semaphore(&self, semaphore: &Semaphore, value: u64) {
        let semaphores = [semaphore.into()];
        let values = [value];
        let wait_info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);

        unsafe { self.handle.wait_semaphores(&wait_info, u64::MAX).unwrap() };
    }

    pub fn create_image(&mut self, create_info: vk::ImageCreateInfo) -> Result<Image, vk::Result> {
        let image = unsafe { self.handle.create_image(&create_info, None)? };
        let mem_requirements = unsafe { self.handle.get_image_memory_requirements(image) };

        let allocation = self
            .allocator
            .allocate(&vulkan::AllocationCreateDesc {
                name: Default::default(),
                requirements: mem_requirements,
                location: gpu_allocator::MemoryLocation::GpuOnly,
                linear: true,
                allocation_scheme: vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .expect("Failed to allocate image");

        unsafe {
            self.handle
                .bind_image_memory(image, allocation.memory(), allocation.offset())?
        };

        Ok(Image {
            usage: create_info.usage,
            format: create_info.format,
            extent: create_info.extent,
            slices: create_info.array_layers,
            levels: create_info.mip_levels,
            allocation: Some(allocation),
            handle: image,
        })
    }

    pub fn create_image_view(
        &self,
        create_info: vk::ImageViewCreateInfo,
    ) -> Result<ImageView, vk::Result> {
        let image_view = unsafe { self.handle.create_image_view(&create_info, None)? };
        Ok(ImageView {
            format: create_info.format,
            subresource_range: create_info.subresource_range,
            handle: image_view,
        })
    }

    pub fn create_sampler(
        &self,
        create_info: vk::SamplerCreateInfo,
    ) -> Result<Sampler, vk::Result> {
        let sampler = unsafe { self.handle.create_sampler(&create_info, None)? };
        Ok(Sampler { handle: sampler })
    }

    pub fn create_buffer(
        &mut self,
        create_info: vk::BufferCreateInfo,
        memory_location: gpu_allocator::MemoryLocation,
    ) -> Result<Buffer, vk::Result> {
        let buffer = unsafe { self.handle.create_buffer(&create_info, None)? };
        let mem_requirements = unsafe { self.handle.get_buffer_memory_requirements(buffer) };

        let allocation = self
            .allocator
            .allocate(&vulkan::AllocationCreateDesc {
                name: Default::default(),
                requirements: mem_requirements,
                location: memory_location,
                linear: true,
                allocation_scheme: vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .expect("Failed to allocate buffer");

        unsafe {
            self.handle
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?
        }

        // Always make sure BDA is requested after `bind_buffer_memory`
        let bda_info = vk::BufferDeviceAddressInfo::default().buffer(buffer);
        let buffer_device_address = unsafe { self.handle.get_buffer_device_address(&bda_info) };

        Ok(Buffer {
            data_size: mem_requirements.size,
            device_address: buffer_device_address,
            allocation,
            handle: buffer,
        })
    }

    pub fn create_swapchain(&self, window: &window::Window) -> Result<SwapChain, vk::Result> {
        let surface = self
            .physical_device
            .create_surface(window)
            .expect("Failed to create surface for swapchain");

        let image_count = self.frame_count.min(surface.capabilities.max_image_count);

        let surface_format = surface
            .formats
            .iter()
            .cloned()
            .find(|&surface_format| surface_format.format == vk::Format::R8G8B8A8_SRGB)
            .unwrap_or(vk::SurfaceFormatKHR::default().format(vk::Format::B8G8R8A8_UNORM));
        let surface_resolution = match surface.capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height,
            },
            _ => surface.capabilities.current_extent,
        };
        let pre_transform = if surface
            .capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface.capabilities.current_transform
        };
        let present_mode = surface
            .present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::IMMEDIATE);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);
        let swapchain = unsafe {
            self.swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Failed to create swapchain")
        };

        let mut acquire_semas = Vec::new();
        for _ in 0..self.frame_count {
            acquire_semas.push(self.create_binary_semaphore()?);
        }

        let mut present_semas = Vec::new();
        for _ in 0..self.frame_count {
            present_semas.push(self.create_binary_semaphore()?);
        }

        Ok(SwapChain {
            format: surface_format.format,
            extent: surface_resolution,
            acquire_semas,
            present_semas,
            surface,
            handle: swapchain,
        })
    }

    pub fn get_swapchain_images(
        &self,
        swapchain: &SwapChain,
    ) -> Result<(Vec<Image>, Vec<ImageView>), vk::Result> {
        let native_images = unsafe {
            self.swapchain_loader
                .get_swapchain_images(swapchain.handle)
                .expect("Cannot get swapchain images")
        };
        let images: Vec<Image> = native_images
            .iter()
            .map(|&image| {
                let extent = vk::Extent3D::default();
                Image {
                    usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                    format: swapchain.format,
                    extent,
                    slices: 1,
                    levels: 1,
                    allocation: None,
                    handle: image,
                }
            })
            .collect();
        let image_views: Vec<ImageView> = images
            .iter()
            .map(|image| {
                let create_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(swapchain.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image.into());
                self.create_image_view(create_info).unwrap()
            })
            .collect();

        Ok((images, image_views))
    }

    pub fn acquire_next_image(
        &self,
        swapchain: &SwapChain,
        acquire_sema: &Semaphore,
    ) -> Result<u32, vk::Result> {
        let (image_id, _suboptimal) = unsafe {
            self.swapchain_loader
                .acquire_next_image(
                    swapchain.handle,
                    u64::MAX,
                    acquire_sema.into(),
                    vk::Fence::null(),
                )
                .expect("Failed to acquire swapchain image")
        };

        // TODO: properly handle suboptimal case
        Ok(image_id)
    }

    pub fn present(
        &self,
        swapchain: &SwapChain,
        present_sema: &Semaphore,
        image_index: u32,
    ) -> Result<bool, vk::Result> {
        let wait_semas = [present_sema.into()];
        let swapchains = [swapchain.into()];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semas)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.swapchain_loader
                .queue_present(self.queue_at(CommandType::Graphics).into(), &present_info)
        }
    }

    pub fn new_frame(&self) -> usize {
        let sema_counter = self.frame_sema.counter as i64;
        let wait_val = std::cmp::max(0, sema_counter - (self.frame_count - 1) as i64) as u64;
        self.wait_for_semaphore(&self.frame_sema, wait_val);

        (self.frame_sema.counter % self.frame_count as u64) as usize
    }

    pub fn end_frame(&mut self) {
        self.frame_sema.advance();
    }

    pub fn submit(
        &self,
        command_queue: &CommandQueue,
        submit_info: vk::SubmitInfo2,
    ) -> Result<(), vk::Result> {
        let submits = [submit_info];
        unsafe {
            self.handle
                .queue_submit2(command_queue.into(), &submits, vk::Fence::null())?
        };

        Ok(())
    }

    pub fn create_command_allocator(
        &self,
        command_type: CommandType,
        flags: vk::CommandPoolCreateFlags,
    ) -> Result<CommandAllocator, vk::Result> {
        let create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(self.queue_at(command_type).family_index)
            .flags(flags);
        let command_allocator = unsafe {
            self.handle
                .create_command_pool(&create_info, None)
                .expect("Failed to create command allocator")
        };

        Ok(CommandAllocator {
            command_type,
            handle: command_allocator,
        })
    }

    pub fn reset_command_allocator(&self, command_allocator: &CommandAllocator) {
        unsafe {
            self.handle
                .reset_command_pool(
                    command_allocator.into(),
                    vk::CommandPoolResetFlags::RELEASE_RESOURCES,
                )
                .expect("Failed to reset command pool")
        };
    }

    pub fn create_command_list(
        &self,
        command_allocator: &CommandAllocator,
    ) -> Result<CommandList, vk::Result> {
        let create_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_allocator.into())
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_list = unsafe {
            self.handle
                .allocate_command_buffers(&create_info)
                .expect("Failed to allocate command list")
        }[0];

        Ok(CommandList {
            command_type: command_allocator.command_type,
            device: self.handle.clone(),
            handle: command_list,
        })
    }

    pub fn begin_command_list(&self, command_list: &CommandList) {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.handle
                .begin_command_buffer(command_list.into(), &begin_info)
                .unwrap()
        };
    }

    pub fn end_command_list(&self, command_list: &CommandList) {
        unsafe { self.handle.end_command_buffer(command_list.into()).unwrap() };
    }
}

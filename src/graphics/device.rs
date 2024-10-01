use ash::vk;
use std::error::Error;
use winit::window;

use super::{QueueType, PhysicalDevice};

pub struct Device {
    pub physical_device: PhysicalDevice,
    pub queues: [vk::Queue; 3],
    pub handle: ash::Device,
}

impl Device {
    pub unsafe fn new(window: &window::Window) -> Result<Self, Box<dyn Error>> {
        let physical_device = PhysicalDevice::new(window)?;
        let handle = physical_device.create_device()?;
        let queues = [
            handle.get_device_queue(physical_device.queue_type_indices[QueueType::Graphics as usize] as u32, 0),
            handle.get_device_queue(physical_device.queue_type_indices[QueueType::Compute  as usize] as u32, 0),
            handle.get_device_queue(physical_device.queue_type_indices[QueueType::Transfer as usize] as u32, 0)
        ];

        println!("Initialized Vulkan for Physicial Device @ 0 ({}-{:?})",
            physical_device.properties.api_version, &physical_device.properties.device_name_as_c_str().unwrap());

        Ok(Self {
            physical_device,
            queues,
            handle,
        })
    }
}

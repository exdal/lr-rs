use ash::vk;
use std::default::Default;

#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CommandType {
    Graphics = 0,
    Transfer = 1,
    Compute = 2,
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct Semaphore {
    pub counter: u64,
    pub handle: vk::Semaphore,
}

impl Semaphore {
    pub fn advance(&mut self) {
        self.counter += 1;
    }
}
define_from!(Semaphore, vk::Semaphore);

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct CommandQueue {
    pub family_index: u32,
    pub semaphore: Semaphore,

    pub handle: vk::Queue,
}
define_from!(CommandQueue, vk::Queue);

pub struct CommandAllocator {
    pub command_type: CommandType,

    pub handle: vk::CommandPool,
}
define_from!(CommandAllocator, vk::CommandPool);

pub struct CommandList {
    pub command_type: CommandType,

    pub device: ash::Device,
    pub handle: vk::CommandBuffer,
}
define_from!(CommandList, vk::CommandBuffer);

impl CommandList {
    pub fn image_barrier(&self, barrier: vk::ImageMemoryBarrier2) {
        let image_barriers = [barrier];
        let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&image_barriers);

        unsafe {
            self.device
                .cmd_pipeline_barrier2(self.into(), &dependency_info)
        };
    }
}

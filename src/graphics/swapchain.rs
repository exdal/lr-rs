use ash::vk;

use super::Semaphore;

pub struct Surface {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,

    pub handle: vk::SurfaceKHR,
}

pub struct SwapChain {
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub acquire_semas: Vec<Semaphore>,
    pub present_semas: Vec<Semaphore>,

    pub surface: Surface,
    pub handle: vk::SwapchainKHR,
}

impl From<SwapChain> for vk::SwapchainKHR {
    fn from(value: SwapChain) -> Self {
        value.handle
    }
}

impl From<&SwapChain> for vk::SwapchainKHR {
    fn from(value: &SwapChain) -> Self {
        value.handle
    }
}

impl SwapChain {
    pub fn frame_semas(&self, frame_counter: u64) -> (&Semaphore, &Semaphore) {
        (
            &self.acquire_semas[frame_counter as usize],
            &self.present_semas[frame_counter as usize],
        )
    }
}

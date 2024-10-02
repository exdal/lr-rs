use ash::vk;
use graphics::CommandType;
#[cfg(target_os = "linux")]
use winit::platform::x11::WindowAttributesExtX11;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowAttributes,
};

use std::{default::Default, error::Error};

mod graphics;

struct Renderer {
    device: graphics::Device,
    swapchain: graphics::SwapChain,
    swapchain_images: Vec<graphics::Image>,
    swapchain_image_views: Vec<graphics::ImageView>,
    command_allocators: Vec<graphics::CommandAllocator>,
    command_lists: Vec<graphics::CommandList>,
}

#[derive(Default)]
struct Application {
    window: Option<winit::window::Window>,
    renderer: Option<Renderer>,
}

impl Application {
    fn draw(&mut self) {
        let renderer = self.renderer.as_mut().unwrap();
        let sema_index = renderer.device.new_frame();
        let frame_sema = &renderer.device.frame_sema;
        let (acquire_sema, present_sema) = renderer.swapchain.frame_semas(sema_index as u64);
        let image_index = renderer
            .device
            .acquire_next_image(&renderer.swapchain, acquire_sema)
            .unwrap();
        let image = &renderer.swapchain_images[image_index as usize];
        let image_view = &renderer.swapchain_image_views[image_index as usize];
        let command_queue = &renderer.device.queue_at(CommandType::Graphics);
        let command_allocator = &renderer.command_allocators[image_index as usize];
        let command_list = &renderer.command_lists[image_index as usize];

        renderer.device.reset_command_allocator(command_allocator);
        renderer.device.begin_command_list(command_list);

        let transition_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
            .dst_access_mask(vk::AccessFlags2::NONE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .subresource_range(image_view.subresource_range)
            .image(image.into());
        command_list.image_barrier(transition_barrier);

        renderer.device.end_command_list(command_list);

        let command_list_infos =
            [vk::CommandBufferSubmitInfo::default().command_buffer(command_list.into())];
        let wait_sema_infos = [vk::SemaphoreSubmitInfo::default()
            .semaphore(acquire_sema.into())
            .stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)];
        let signal_sema_infos = [
            vk::SemaphoreSubmitInfo::default()
                .semaphore(present_sema.into())
                .stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE),
            vk::SemaphoreSubmitInfo::default()
                .semaphore(frame_sema.into())
                .value(frame_sema.counter + 1)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS),
        ];

        let submit_info = vk::SubmitInfo2::default()
            .wait_semaphore_infos(&wait_sema_infos)
            .signal_semaphore_infos(&signal_sema_infos)
            .command_buffer_infos(&command_list_infos);
        renderer.device.submit(command_queue, submit_info).unwrap();
        renderer.device.end_frame();
        renderer
            .device
            .present(&renderer.swapchain, present_sema, image_index)
            .unwrap();
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Lorr")
            .with_inner_size(winit::dpi::LogicalSize::new(
                f64::from(1580),
                f64::from(820),
            ))
            .with_resizable(false)
            .with_name("Lorr", "");

        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to create window");
        let device = graphics::Device::new(3).unwrap();
        let swapchain = device.create_swapchain(&window).unwrap();
        let (swapchain_images, swapchain_image_views) =
            device.get_swapchain_images(&swapchain).unwrap();
        let mut command_allocators = Vec::new();
        (0..device.frame_count).for_each(|_| {
            command_allocators.push(
                device
                    .create_command_allocator(
                        CommandType::Graphics,
                        vk::CommandPoolCreateFlags::default(),
                    )
                    .unwrap(),
            );
        });

        let mut command_lists = Vec::new();
        (0..device.frame_count).for_each(|i| {
            command_lists.push(
                device
                    .create_command_list(&command_allocators[i as usize])
                    .unwrap(),
            );
        });

        self.window = Some(window);
        self.renderer = Some(Renderer {
            device,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            command_allocators,
            command_lists,
        })
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("exit");
                event_loop.exit()
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        self.window.as_ref().unwrap().request_redraw();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = Application::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

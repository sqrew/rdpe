mod shader;
mod window;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = window::App::new();
    event_loop.run_app(&mut app).unwrap();
}

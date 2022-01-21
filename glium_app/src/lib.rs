
use std::sync::Mutex;

use egui_winit::winit::event_loop::EventLoop;

use context::Context;

extern crate glium;
extern crate async_trait;
use async_trait::async_trait;

pub mod context;
pub mod timer;
pub mod io;


pub use timer::Timer;

pub use glium::{
    glutin::{
        dpi::{PhysicalSize, Size},
        window::WindowBuilder,
        ContextBuilder,
    },
    *
};
use tokio::runtime::Runtime;

#[async_trait]
pub trait Application {
    fn launch_settings(&self) -> WindowBuilder;
    fn init(&mut self, ctx: &mut Context);
    async fn update(&mut self, t: &Timer);
    fn render(&mut self, ctx: &mut Context);
    fn close(&mut self);
}

pub fn run<A: 'static + Application>(mut app: A, rt: Runtime) {
    let event_loop = EventLoop::new();
    let wb = app.launch_settings();
    let cb = ContextBuilder::new().with_vsync(false);
    let display = Display::new(wb, cb, &event_loop).expect("Failed to open Display!");

    let mut egui_glium = egui_glium::EguiGlium::new(&display);

    let mut context: Context = Context::new(display, egui_glium);
    let mut t = Timer::new();

    t.reset();
    // event_loop.run_async(async move |ev, _, control_flow| {
    event_loop.run( move |ev, _, control_flow| {

        use glutin::event::WindowEvent;

        // Handle our own events
        let mut events_cleared = false;
        use glutin::event::{Event::*, *};
        match &ev {
            glutin::event::Event::WindowEvent { event, .. } => 
            {
                let consume = context.gui.on_event(&event);

                match event {
                    WindowEvent::CloseRequested => {
                        app.close();
                        *control_flow = glutin::event_loop::ControlFlow::Exit;
                    }
                    WindowEvent::CursorMoved {
                        device_id,
                        position,
                        ..
                    } => {
                        context.mouse.update_pos((position.x as i32, position.y as i32));
                    }
                    WindowEvent::MouseInput {
                        device_id,
                        state,
                        button,
                        ..
                    } => {
                        let mut mbutton: u16 = 0;
                        match button {
                            MouseButton::Left => {
                                mbutton = 0;
                            }
                            MouseButton::Middle => {
                                mbutton = 1;
                            }
                            MouseButton::Right => {
                                mbutton = 2;
                            }
                            MouseButton::Other(bnum) => {
                                if bnum > &(9 as u16) {
                                    return;
                                }
                                mbutton = *bnum;
                            }
                        }
                        let mut pressed = false;
                        if state == &ElementState::Pressed {
                            pressed = true;
                        }
                        if pressed {
                            context.mouse.press_button(mbutton as usize);
                        } else {
                            context.mouse.release_button(mbutton as usize);
                        }
                    }
                    WindowEvent::MouseWheel {
                        device_id, delta, ..
                    } => match delta {
                        MouseScrollDelta::LineDelta(y, x) => {
                            context.mouse.scroll((*x, *y));
                        }
                        _ => {}
                    },
                    WindowEvent::KeyboardInput {
                        device_id,
                        input,
                        is_synthetic,
                        ..
                    } => match input {
                        KeyboardInput {
                            scancode: _,
                            state,
                            virtual_keycode,
                            ..
                        } => match virtual_keycode {
                            None => {}
                            Some(key) => {
                                if state == &ElementState::Pressed {
                                    context.keyboard.press(*key);
                                } else {
                                    context.keyboard.release(*key);
                                }
                            }
                        },
                    },
                    _ => {}
            }
            },
            MainEventsCleared => {
                events_cleared = true;
            }
            RedrawEventsCleared => {}
            NewEvents(cause) => match cause {
                StartCause::Init => {
                    app.init(&mut context);
                }
                _ => {}
            },
            _ => {}
        }

        if !events_cleared {
            return;
        }

        // Update
        match t.go() {
            None => {}
            Some(_) => {

                rt.block_on(async {
                    app.update(&t).await;
                });
                // app.update(&t, &rt);

                app.render(&mut context);

                context.mouse.next_frame();
                context.keyboard.next_frame();
            }
        }
    });
}
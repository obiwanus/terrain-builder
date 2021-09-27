// #![allow(dead_code)]
// #![allow(unused)]

extern crate gl as opengl_lib;

mod camera;
mod editor;
mod input;
mod opengl;
mod ray;
mod skybox;
mod terrain;
mod texture;
mod utils;

use std::error::Error;
use std::time::Instant;

use egui::{Event as GuiEvent, Pos2, RawInput as EguiInput, Rect};
use glam::{Mat4, Vec2, Vec3};
use glutin::event::{
    DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
    WindowEvent,
};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::{Api, GlProfile, GlRequest};
use glutin::{PossiblyCurrent, WindowedContext};

use camera::Camera;
use editor::gui::Gui;
use input::{vec2_to_egui_pos2, vec2_to_egui_vec2, vkeycode_to_egui_key, Input, Modifiers};
use opengl_lib::types::GLuint;
use skybox::Skybox;
use terrain::Terrain;
use utils::vec2_infinity;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// ==================================== Main loop =================================================

fn main() {
    let event_loop = EventLoop::new();
    let mut game = Game::new(&event_loop).unwrap_or_else(|error| {
        eprintln!("{}", error);
        std::process::exit(1);
    });

    event_loop.run(move |event, _, control_flow| {
        if let Err(error) = game.process_event(event, control_flow) {
            eprint!("{}", error);
            std::process::exit(1);
        };
    });
}

// ==================================== Game ======================================================

static mut WINDOW_WIDTH: usize = 0;
static mut WINDOW_HEIGHT: usize = 0;

struct DirectionalLight {
    color: Vec3,
    direction: Vec3,
}

#[derive(Clone)]
enum GameMode {
    Game,
    Editor {
        state: EditorState,
        mode: EditorMode,
    },
    Menu,
}

#[derive(Clone)]
enum EditorMode {
    General,
    Terrain { tool: TerrainTool },
}

#[derive(Clone)]
struct EditorState {
    free_camera: bool,
}

#[derive(Clone)]
enum TerrainTool {
    Sculpt,
    PaintTextures,
    PaintTrees,
    PaintVegetation,
}

// NOTE: no need to worry about std140 because Mat4's are aligned properly and with no gaps
#[repr(C)]
pub struct TransformsUBO {
    mvp: Mat4,
    proj: Mat4,
    view: Mat4,
    model: Mat4, // still unsure whether it belongs here
    sun_vp: Mat4,
}

struct Game {
    windowed_context: WindowedContext<PossiblyCurrent>,
    in_focus: bool,

    game_start: Instant,
    frame_start: Instant,

    screen_size: Vec2, // in logical pixels
    scale_factor: f32,

    old_input: Input,
    input: Input,

    gui_input: EguiInput,
    gui: Gui,

    camera: Camera,

    terrain: Terrain,
    skybox: Skybox,

    mode: GameMode,

    // tmp
    transforms_ubo: GLuint,
    transforms_data: TransformsUBO,
}

impl Game {
    /// Creates a window and inits a new game
    fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        // Create window

        #[cfg(all(windows))]
        let window_builder = {
            let monitor = event_loop.primary_monitor().unwrap_or_else(|| {
                event_loop
                    .available_monitors()
                    .next()
                    .expect("Couldn't find monitor")
            });
            let inner_size = glutin::dpi::LogicalSize::new(
                monitor.size().width - 100,
                monitor.size().height - 100,
            );

            WindowBuilder::new()
                .with_title("Мёртвый трилистник")
                .with_resizable(false)
                .with_position(glutin::dpi::LogicalPosition::new(70, 10))
                .with_inner_size(inner_size)
        };

        #[cfg(not(windows))]
        let window_builder = WindowBuilder::new()
            .with_title("Мёртвый трилистник")
            .with_position(glutin::dpi::LogicalPosition::new(70, 10))
            // .with_fullscreen(Some(glutin::window::Fullscreen::Borderless(
            //     event_loop.primary_monitor(),
            // )))
            .with_inner_size(glutin::dpi::LogicalSize::new(1920, 1080))
            .with_resizable(false);

        let gl_request = GlRequest::Specific(Api::OpenGl, (4, 5));
        let gl_profile = GlProfile::Core;
        let windowed_context = glutin::ContextBuilder::new()
            .with_gl(gl_request)
            .with_gl_profile(gl_profile)
            .with_srgb(true)
            .with_double_buffer(Some(true))
            .with_depth_buffer(16)
            .with_vsync(true)
            .build_windowed(window_builder, &event_loop)?;

        // Set up OpenGL
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };
        gl::load_with(|s| windowed_context.get_proc_address(s) as *const _);
        let window = windowed_context.window();
        // window.set_cursor_grab(true)?;
        // window.set_cursor_visible(false);
        let window_size = window.inner_size();
        unsafe {
            // Remember window dimensions for further viewport adjustments
            WINDOW_WIDTH = window_size.width as usize;
            WINDOW_HEIGHT = window_size.height as usize;

            gl::Viewport(0, 0, window_size.width as i32, window_size.height as i32);
            gl::ClearColor(0.05, 0.05, 0.05, 1.0);
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::FRAMEBUFFER_SRGB);
            gl::Enable(gl::CULL_FACE);

            #[cfg(not(target_os = "macos"))]
            {
                // MacOS deprecated OpenGL, which is stuck at 4.1 so no debug callbacks here :(
                gl::Enable(gl::DEBUG_OUTPUT);
                gl::DebugMessageCallback(Some(opengl::debug_callback), std::ptr::null());
            }
        }

        // // Directional light
        // let light_color = Vec3::new(1.0, 0.7, 0.7);
        // shader.set_vec3("directional_light.ambient", &(0.2f32 * light_color))?;
        // shader.set_vec3("directional_light.diffuse", &(0.5f32 * light_color))?;
        // shader.set_vec3("directional_light.specular", &(1.0f32 * light_color))?;

        // // Default terrain material
        // shader.set_vec3("material.specular", &Vec3::new(0.4, 0.4, 0.4))?;
        // shader.set_float("material.shininess", 10.0)?;

        // Set up camera
        let camera = Camera::new(
            Vec3::new(520.0, 250.0, 100.0),
            Vec3::new(0.0, 130.0, 0.0),
            window_size.width,
            window_size.height,
        );

        // Set up camera transforms uniform buffer
        let mut transforms_ubo: GLuint = 0;
        unsafe {
            gl::CreateBuffers(1, &mut transforms_ubo);
            gl::NamedBufferStorage(
                transforms_ubo,
                std::mem::size_of::<TransformsUBO>() as isize,
                std::ptr::null(),
                gl::DYNAMIC_STORAGE_BIT,
            );
            gl::BindBufferBase(gl::UNIFORM_BUFFER, 1, transforms_ubo);
        }
        let transforms_data = {
            let proj = camera.get_projection_matrix();
            let view = camera.get_view_matrix();
            let model = Mat4::IDENTITY;
            let sun_proj = Mat4::orthographic_rh_gl(-600.0, 600.0, -600.0, 600.0, 1.0, 1200.0);
            let sun_view = Mat4::look_at_rh(
                Vec3::new(0.0, 200.0, 500.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            );

            TransformsUBO {
                mvp: proj * view * model,
                proj,
                view,
                model,
                sun_vp: sun_proj * sun_view,
            }
        };

        let terrain = Terrain::new(Vec2::new(0.0, 0.0))?;

        let skybox = Skybox::from([
            "textures/skybox/default/right.png",
            "textures/skybox/default/left.png",
            "textures/skybox/default/top.png",
            "textures/skybox/default/bottom.png",
            "textures/skybox/default/front.png",
            "textures/skybox/default/back.png",
        ])?;

        let scale_factor = window.scale_factor() as f32;
        let screen_size_physical = Vec2::new(window_size.width as f32, window_size.height as f32);
        let screen_size_logical = screen_size_physical / scale_factor;

        // Gui and its initial input
        let gui = Gui::new(screen_size_physical)?;
        let gui_input = EguiInput {
            screen_rect: Some(Rect::from_min_max(
                Pos2::new(0.0, 0.0),
                Pos2::new(screen_size_logical.x, screen_size_logical.y),
            )),
            pixels_per_point: Some(scale_factor),
            time: Some(0.0),

            ..Default::default()
        };

        let now = Instant::now();
        let input = Input {
            camera_moved: true,
            ..Default::default()
        };

        Ok(Game {
            windowed_context,

            game_start: now,
            frame_start: now,

            screen_size: screen_size_logical,
            scale_factor,

            old_input: Input::default(),
            input,

            gui_input,
            gui,

            camera,
            in_focus: true,

            terrain,
            skybox,

            mode: GameMode::Editor {
                state: EditorState { free_camera: false },
                mode: EditorMode::Terrain {
                    tool: TerrainTool::Sculpt,
                },
            },

            transforms_ubo,
            transforms_data,
        })
    }

    fn process_event(&mut self, event: Event<()>, control_flow: &mut ControlFlow) -> Result<()> {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => self.input.should_exit = true,
                WindowEvent::ScaleFactorChanged {
                    scale_factor,
                    new_inner_size: _,
                } => {
                    self.scale_factor = scale_factor as f32;
                    self.gui_input.pixels_per_point = Some(scale_factor as f32);
                }
                WindowEvent::ModifiersChanged(state) => {
                    self.input.modifiers = Modifiers {
                        alt: state.alt(),
                        ctrl: state.ctrl(),
                        shift: state.shift(),
                        logo: state.logo(),
                    };
                    self.gui_input.modifiers = egui::Modifiers {
                        alt: state.alt(),
                        ctrl: state.ctrl(),
                        shift: state.shift(),
                        mac_cmd: false,
                        command: state.ctrl(),
                    };
                    #[cfg(target_os = "macos")]
                    {
                        self.gui_input.modifiers.mac_cmd = state.logo();
                        self.gui_input.modifiers.command = state.logo();
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pointer =
                        Vec2::new(position.x as f32, position.y as f32) / self.scale_factor;
                    self.input.pointer = pointer;
                    self.input.pointer_moved = true;
                    self.gui_input
                        .events
                        .push(GuiEvent::PointerMoved(vec2_to_egui_pos2(pointer)));
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    let pressed = state == ElementState::Pressed;

                    match button {
                        MouseButton::Left => self.input.mouse_buttons.primary = pressed,
                        MouseButton::Right => self.input.mouse_buttons.secondary = pressed,
                        MouseButton::Middle => self.input.mouse_buttons.middle = pressed,
                        _ => {}
                    }

                    let button = match button {
                        MouseButton::Left => Some(egui::PointerButton::Primary),
                        MouseButton::Right => Some(egui::PointerButton::Secondary),
                        MouseButton::Middle => Some(egui::PointerButton::Middle),
                        _ => None,
                    };
                    if let Some(button) = button {
                        self.gui_input.events.push(GuiEvent::PointerButton {
                            pos: vec2_to_egui_pos2(self.input.pointer),
                            button,
                            pressed,
                            modifiers: self.gui_input.modifiers,
                        });
                    }
                }
                WindowEvent::Focused(focused) => {
                    self.in_focus = focused;
                    self.input.modifiers = Modifiers::default();
                    self.gui_input.modifiers = egui::Modifiers::default();
                    // @idea: Try using Wait here?
                }
                WindowEvent::ReceivedCharacter(ch) => {
                    if is_printable_char(ch)
                        && !self.gui_input.modifiers.ctrl
                        && !self.gui_input.modifiers.mac_cmd
                    {
                        self.gui_input.events.push(GuiEvent::Text(ch.to_string()));
                    }
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(virtual_key_code),
                            ..
                        },
                    ..
                } => {
                    let pressed = state == ElementState::Pressed;

                    match virtual_key_code {
                        VirtualKeyCode::W => self.input.forward = pressed,
                        VirtualKeyCode::A => self.input.left = pressed,
                        VirtualKeyCode::S => self.input.back = pressed,
                        VirtualKeyCode::D => self.input.right = pressed,
                        _ => {}
                    }

                    if let Some(key) = vkeycode_to_egui_key(virtual_key_code) {
                        self.gui_input.events.push(GuiEvent::Key {
                            key,
                            pressed,
                            modifiers: self.gui_input.modifiers,
                        });
                    }
                }
                _ => {}
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } if self.in_focus => {
                    let (x, y) = delta;
                    let delta = Vec2::new(x as f32, y as f32) / self.scale_factor;
                    self.input.pointer_delta += delta;
                    self.input.pointer_moved = true;
                }
                DeviceEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(x, y),
                } => {
                    let scroll_delta = Vec2::new(x, y) / self.scale_factor;
                    self.input.scroll_delta += scroll_delta;
                    self.input.scrolled = true;
                    self.gui_input.scroll_delta = vec2_to_egui_vec2(scroll_delta)
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                if !self.input.should_exit {
                    self.update_and_render()?;
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        };
        Ok(())
    }

    fn update_and_render(&mut self) -> Result<()> {
        let now = Instant::now();
        let delta_time = now.duration_since(self.frame_start).as_secs_f32();
        self.frame_start = now;
        let time = now.duration_since(self.game_start).as_secs_f64();
        self.gui_input.time = Some(time);
        self.input.time = time as f32;

        let new_mode = match self.mode.clone() {
            GameMode::Menu => unimplemented!("Menu is not implemented"),
            GameMode::Game => unimplemented!("Game mode is not implemented"),
            GameMode::Editor { mode, state } => self.draw_editor(delta_time, mode, state)?,
        };

        self.mode = new_mode;

        Ok(())
    }

    fn draw_editor(
        &mut self,
        delta_time: f32,
        mut mode: EditorMode,
        mut state: EditorState,
    ) -> Result<GameMode> {
        let (should_exit, gui_shapes) = self.gui.layout_and_interact(self.gui_input.take());

        if should_exit {
            self.input.should_exit = true;
        }

        if self.gui.wants_input() {
            // Pointer over UI or currently interacting with it
            self.terrain.cursor = vec2_infinity(); // hide terrain cursor
            self.windowed_context.window().set_cursor_visible(true); // we always want cursor with UI
        } else {
            // Process input
            state.free_camera = self.input.mouse_buttons.secondary;
            self.camera.speed_boost = self.input.modifiers.shift;

            // Move camera
            if state.free_camera {
                use camera::Movement::*;
                if self.input.forward {
                    self.camera.go(Forward, delta_time);
                    self.input.camera_moved = true;
                }
                if self.input.left {
                    self.camera.go(Left, delta_time);
                    self.input.camera_moved = true;
                }
                if self.input.back {
                    self.camera.go(Backward, delta_time);
                    self.input.camera_moved = true;
                }
                if self.input.right {
                    self.camera.go(Right, delta_time);
                    self.input.camera_moved = true;
                }

                // Rotate camera
                if self.input.pointer_moved {
                    let delta = self.input.pointer_delta;
                    self.camera.rotate(delta.x, delta.y);
                    self.input.camera_moved = true;
                }
            }

            if self.input.camera_moved {
                // Update camera tranforms uniform buffer
                self.transforms_data.view = self.camera.get_view_matrix();
                self.transforms_data.proj = self.camera.get_projection_matrix();
                self.transforms_data.mvp = self.transforms_data.proj
                    * self.transforms_data.view
                    * self.transforms_data.model;
                let data = &self.transforms_data as *const TransformsUBO;
                unsafe {
                    gl::NamedBufferSubData(
                        self.transforms_ubo,
                        0,
                        std::mem::size_of::<TransformsUBO>() as isize,
                        data as *const _,
                    )
                }
            }

            if self.input.pointer_moved || self.input.camera_moved {
                let ray = self.camera.get_ray_through_pixel(self.input.pointer);
                if let Some(point) = self.terrain.intersect_with_ray(&ray) {
                    self.terrain.cursor = Vec2::new(point.x, point.z);
                } else {
                    self.terrain.cursor = vec2_infinity();
                }
            }

            if self.input.scrolled {
                let y = self.input.scroll_delta.y;
                self.terrain.brush.size = (self.terrain.brush.size - y * 1.5).clamp(0.1, 200.0);
                // self.terrain.tess_level = (self.terrain.tess_level - y * 0.2).clamp(1.0, 16.0);
            }

            if self.input.mouse_buttons.primary && self.terrain.cursor.is_finite() {
                self.terrain
                    .shape_terrain(delta_time, !self.input.modifiers.ctrl);
            }
        }

        // Draw
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        self.terrain.draw(self.input.time)?;
        self.skybox.draw()?;

        self.gui.draw(gui_shapes);

        self.windowed_context.swap_buffers()?;

        // Clear old input
        self.old_input = self.input.renew();

        Ok(GameMode::Editor { state, mode })
    }
}

/// Winit sends special keys (backspace, delete, F1, ...) as characters.
/// Ignore those.
/// We also ignore '\r', '\n', '\t'.
/// Newlines are handled by the `Key::Enter` event.
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}

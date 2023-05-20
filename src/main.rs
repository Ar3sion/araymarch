mod camera;

use spin_sleep::LoopHelper;
use glutin::event_loop::{EventLoop, ControlFlow};
use glutin::window::{WindowBuilder, Fullscreen};
use glutin::{GlRequest, ContextBuilder, Api};
use glutin::event::{Event, WindowEvent, DeviceEvent, ElementState, MouseScrollDelta};
use std::ffi::CStr;
use core::{ptr, mem};
use glutin::dpi::PhysicalSize;
use cgmath::{Matrix4, Matrix, Vector2};
use crate::camera::Camera;

fn main() {
    const FRAME_RATE: f32 = 61.0;
    
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_title("Araymarch")
        .with_fullscreen(Some(Fullscreen::Borderless(event_loop.primary_monitor())));
    let windowed_context = unsafe { ContextBuilder::new()
        .with_vsync(true)
        .with_gl(GlRequest::Specific(Api::OpenGl, (4, 3))) // 4.3 for compute shaders
        .build_windowed(window_builder, &event_loop)
        .unwrap()
        .make_current()
        .unwrap() };
    windowed_context.window().set_cursor_grab(true).unwrap();
    windowed_context.window().set_cursor_visible(false);
    gl::load_with(|s| windowed_context.get_proc_address(s));
    
    print_workgroup_capabilities();
    let mut texture = None;
    let compute_shader = compile_compute_shader(include_str!("../compute.glsl")).unwrap();
    let quad_vao = create_quad_vao();
    let quad_shader = compile_shader(include_str!("../quad.vert"), include_str!("../quad.frag")).unwrap();
    
    let mut camera = Camera::new();
    let mut focused = true;
    
    let mut timer = LoopHelper::builder().build_with_target_rate(FRAME_RATE);
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Focused(new_focused) => {
                        focused = new_focused;
                        let window = windowed_context.window();
                        window.set_cursor_grab(focused).unwrap();
                        window.set_cursor_visible(!focused);
                    }
                    WindowEvent::KeyboardInput {input, .. } => {
                        match input.state {
                            ElementState::Pressed => if let Some(code) = input.virtual_keycode { 
                                camera.key_pressed(code) 
                            },
                            ElementState::Released => if let Some(code) = input.virtual_keycode {
                                camera.key_released(code) 
                            }
                        }
                    },
                    WindowEvent::MouseWheel { delta, .. } => {
                        let delta = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(physical_position) => physical_position.y as f32
                        };
                        camera.scroll_wheel(delta);
                    }
                    _ => {}
                }
            },
            Event::DeviceEvent { event, .. } => {
                match event {
                    DeviceEvent::MouseMotion { delta: (delta_x, delta_y), .. } => {
                        if focused {
                            camera.mouse_movement(Vector2::new(delta_x as f32, delta_y as f32));
                        }
                    },
                    _ => {}
                }
            }
            Event::MainEventsCleared => {
                /******** Loop begins ********/
                let delta_time = timer.loop_start();
                camera.update_position(delta_time);
                
                let (texture, width, height) = if let Some(texture) = texture {
                    texture
                } else {
                    let PhysicalSize { width, height } = windowed_context.window().inner_size();
                    texture = Some((create_texture(width, height), width, height));
                    texture.unwrap()
                };
                
                unsafe {
                    let workgroups_width = ceil_div(width, 16);
                    let workgroups_height = ceil_div(height, 16);
                    
                    gl::UseProgram(compute_shader);
                    load_matrix4(0, camera.get_transformation());
                    
                    gl::BindImageTexture(0, texture, 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::RGBA32F);
                    
                    gl::DispatchCompute(workgroups_width, workgroups_height, 1);
                    gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
                    
                    gl::BindImageTexture(0, 0, 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::RGBA32F);
                    gl::UseProgram(0);
                }
                
                unsafe {
                    gl::UseProgram(quad_shader);
                    gl::BindVertexArray(quad_vao);
                    gl::ActiveTexture(gl::TEXTURE0);
                    gl::BindTexture(gl::TEXTURE_2D, texture);
                    
                    gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
                    
                    gl::BindTexture(gl::TEXTURE_2D, 0);
                    gl::ActiveTexture(0);
                    gl::BindVertexArray(0);
                    gl::UseProgram(0);
                }
                
                windowed_context.swap_buffers().unwrap();
                timer.loop_sleep();
                /******** Loop ends ********/
            }
            _ => {}
        }
    });
}

fn print_workgroup_capabilities() {
    let mut workgroup_count = [0; 3];
    let mut workgroup_size = [0; 3];
    let mut workgroup_invocations = 0;
    
    unsafe {
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_COUNT, 0, &mut workgroup_count[0] as *mut i32);
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_COUNT, 1, &mut workgroup_count[1] as *mut i32);
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_COUNT, 2, &mut workgroup_count[2] as *mut i32);
    }
    
    println!("Max workgroup count: \n x: {} \n y: {} \n z: {}", workgroup_count[0], workgroup_count[1], workgroup_count[2]);

    unsafe {
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 0, &mut workgroup_size[0] as *mut i32);
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 1, &mut workgroup_size[1] as *mut i32);
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 2, &mut workgroup_size[2] as *mut i32);
    }
    
    println!("Max workgroup size: \n x: {} \n y: {} \n z: {}", workgroup_size[0], workgroup_size[1], workgroup_size[2]);
    
    unsafe {
        gl::GetIntegerv(gl::MAX_COMPUTE_WORK_GROUP_INVOCATIONS, &mut workgroup_invocations as *mut i32);
    }
    
    println!("Max workgroup invocations: {}", workgroup_invocations);
}

fn create_texture(width: u32, height: u32) -> u32 {
    unsafe {
        let mut texture = 0;
        gl::GenTextures(1, &mut texture as *mut u32);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA32F as i32, width as i32, height as i32, 0, gl::RGBA, gl::FLOAT, ptr::null());
        gl::BindTexture(gl::TEXTURE_2D, 0);
        texture
    }
}

fn create_quad_vao() -> u32 {
    const VERTICES: [f32; 8] = [
        -1.0, 1.0,
        1.0, 1.0,
        -1.0, -1.0,
        1.0, -1.0
    ];
    
    unsafe {
        let mut quad_vbo = 1;
        gl::GenBuffers(1, &mut quad_vbo as *mut u32);
        gl::BindBuffer(gl::ARRAY_BUFFER, quad_vbo);
        gl::BufferData(gl::ARRAY_BUFFER, mem::size_of_val(&VERTICES) as isize, (&VERTICES as *const f32).cast(), gl::STATIC_DRAW);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        
        let mut quad_vao = 1;
        gl::GenVertexArrays(1, &mut quad_vao as *mut u32);
        gl::BindVertexArray(quad_vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, quad_vbo);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());
        gl::EnableVertexAttribArray(0);
        gl::BindVertexArray(0);
        
        quad_vbo
    }
}

fn create_shader(source: &str, shader_type: u32) -> Result<u32, String> {
    let shader = unsafe { gl::CreateShader(shader_type) };
    let text_ptr: *const i8 = source.as_ptr().cast();
    let len = source.len() as i32;
    unsafe { gl::ShaderSource(shader, 1, &text_ptr as *const *const i8, &len as *const i32); }
    unsafe { gl::CompileShader(shader); }
    let mut compile_status: i32 = 0;
    unsafe { gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut compile_status as *mut i32); }
    if compile_status == gl::FALSE.into() {
        let mut log_len: i32 = 0;
        unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_len as *mut i32); }
        let mut log_buf = vec![0u8; log_len as usize];
        unsafe { gl::GetShaderInfoLog(shader, log_len, ptr::null_mut(), log_buf.as_mut_ptr().cast()); }
        Err(CStr::from_bytes_with_nul(&log_buf).expect("Invalid string!").to_string_lossy().into_owned())
    } else {
        Ok(shader)
    }
}

fn compile_shader(vertex_source: &str, fragment_source: &str) -> Result<u32, String> {
    let vertex_shader = create_shader(vertex_source, gl::VERTEX_SHADER)?;
    let fragment_shader = create_shader(fragment_source, gl::FRAGMENT_SHADER)?;
    let program = unsafe { gl::CreateProgram() };
    unsafe {
        gl::AttachShader(program, vertex_shader);
        gl::AttachShader(program, fragment_shader);
        gl::LinkProgram(program);
    }
    check_link_status(program)?;
    unsafe {
        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);
    }
    Ok(program)
}

fn compile_compute_shader(source: &str) -> Result<u32, String> {
    let compute_shader = create_shader(source, gl::COMPUTE_SHADER)?;
    let program = unsafe { gl::CreateProgram() };
    unsafe {
        gl::AttachShader(program, compute_shader);
        gl::LinkProgram(program);
    }
    check_link_status(program)?;
    unsafe {
        gl::DeleteShader(compute_shader);
    }
    Ok(program)
}

fn check_link_status(program: u32) -> Result<(), String> {
    let mut link_status: i32 = 0;
    unsafe { 
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut link_status); 
    }
    if link_status == gl::FALSE.into() {
        let mut log_len: i32 = 0;
        unsafe { gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut log_len); }
        let mut log_buf = vec![0u8; log_len as usize];
        unsafe { gl::GetProgramInfoLog(program, log_len, ptr::null_mut(), log_buf.as_mut_ptr().cast()); }
        Err(CStr::from_bytes_with_nul(&log_buf).unwrap().to_string_lossy().into_owned())
    } else {
        Ok(())
    }
}

fn ceil_div(a: u32, b: u32) -> u32 {
    (a + b - 1) / b
}

fn load_matrix4(location: i32, matrix: Matrix4<f32>) {
    unsafe {
        gl::UniformMatrix4fv(location, 1, gl::FALSE, matrix.as_ptr());
    }
}
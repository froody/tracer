extern crate gl;
extern crate glutin;
use gl::types::*;
use std::ffi::{CStr, CString};
use std::{mem, ptr, str};
use glutin::event::{VirtualKeyCode, ElementState};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

static VS_SRC: &'static str = "
#version 450 core
in vec3 position;
void main() {
    gl_Position = vec4(position, 1.0);
}";

static FS_SRC: &'static str = "
#version 450 core
out vec4 out_color;
void main() {
    out_color = vec4(1.0, 1.0, 1.0, 1.0);
}";

static GS_SRC: &'static str = "
#version 450 core
layout (points) in;
layout (triangle_strip, max_vertices = 256) out;
void main() {
    for (int i = 0; i < 1; i++) 
    {
            //gl_Position = gl_in[i].gl_Position;
            //EmitVertex();
            //continue;

        //gl_Position = gl_in[i].gl_Position;
        vec4 position = gl_in[i].gl_Position;
        gl_Position = vec4(position.x - position.z * 0.05, position.y - 0.1, 0.0, position.w);
        EmitVertex();
        gl_Position = vec4(position.x - position.z * 0.05, position.y + 0.1, 0.0, position.w);
        EmitVertex();
        gl_Position = vec4(position.x + position.z * 0.05, position.y - 0.1, 0.0, position.w);
        EmitVertex();
        gl_Position = vec4(position.x + position.z * 0.05, position.y + 0.1, 0.0, position.w);
        EmitVertex();
        EndPrimitive();
    }
}";

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        shader = gl::CreateShader(ty);
        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(
                shader,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                str::from_utf8(&buf)
                    .ok()
                    .expect("ShaderInfoLog not valid utf8")
            );
        }
    }
    shader
}

fn link_program(vs: GLuint, fs: GLuint, gs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::AttachShader(program, gs);
        gl::LinkProgram(program);
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(
                program,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                str::from_utf8(&buf)
                    .ok()
                    .expect("ProgramInfoLog not valid utf8")
            );
        }
        program
    }
}

pub fn render_loop(rects_cb: impl Fn() -> Vec<f32> + 'static) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("taco");
    //build(&event_loop).unwrap();
    let windowed_context = glutin::ContextBuilder::new()
        .build_windowed(window, &event_loop)
        .unwrap();
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };
    let gl = gl::load_with(|s| windowed_context.context().get_proc_address(s) as *const _);
    // Create GLSL shaders
    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
    let gs = compile_shader(GS_SRC, gl::GEOMETRY_SHADER);
    let program = link_program(vs, fs, gs);
    let mut vao = 0;
    let mut vbo = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        let VERTEX_DATA: Vec<GLfloat> = vec![0.0, 0.5, 1.0, 0.5, -0.5, 2.0, -0.5, -0.5, 0.5];
        //static VERTEX_DATA: [GLfloat; 9] = [-1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 0.0, 1.0, 0.0];
        gl::GenBuffers(1, &mut vbo);
        gl::UseProgram(program);
        gl::BindFragDataLocation(program, 0, CString::new("out_color").unwrap().as_ptr());
        let mut max_vertices = 0;
        gl::GetIntegerv(gl::MAX_GEOMETRY_OUTPUT_VERTICES, &mut max_vertices);
        println!("max vertices: {}", max_vertices);

        // Specify the layout of the vertex data
        let pos_attr = gl::GetAttribLocation(program, CString::new("position").unwrap().as_ptr());
        gl::EnableVertexAttribArray(pos_attr as GLuint);
        gl::VertexAttribPointer(
            pos_attr as GLuint,
            3,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            0,
            ptr::null(),
        );
    }
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } => match event {
            WindowEvent::CloseRequested if window_id == windowed_context.window().id() => {
                *control_flow = ControlFlow::Exit
            }
            WindowEvent::RedrawRequested => unsafe {
                let rects = rects_cb();
                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (rects.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                    mem::transmute(&rects[0]),
                    gl::STATIC_DRAW,
                );
                gl::ClearColor(0.0, 0.0, 1.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
                let version = gl::GetString(gl::VERSION);
                println!("redraw yo! {:?}", CStr::from_ptr(version as *const i8));
                // Use shader program
                gl::DrawArrays(gl::POINTS, 0, (rects.len() / 3) as i32);

                windowed_context.swap_buffers().unwrap();
            },
            WindowEvent::KeyboardInput{device_id, input} => {
                if input.state == ElementState::Pressed {
                match input.virtual_keycode {
                    Some(keycode) => {
                        match keycode {
                            VirtualKeyCode::W => {println!("got W")}
                            VirtualKeyCode::A => {println!("got A")}
                            VirtualKeyCode::S => println!("got S"),
                            VirtualKeyCode::D => println!("got D"),
                            _ => ()
                        }
                    }
                    _ => ()
                }
                }
                *control_flow = ControlFlow::Wait
            }
            _ => *control_flow = ControlFlow::Wait,
        },
        _ => *control_flow = ControlFlow::Wait,
    });
}
extern crate clap;
extern crate json;
mod trace;
use std::ffi::{CStr, CString};
use std::sync::atomic::Ordering;
use std::{mem, ptr, str};
use trace::trace_types::TraceEventType;

use clap::{App, Arg};

extern crate gl;
extern crate glutin;
//use glutin::{self, PossiblyCurrent};
use gl::types::*;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

static VS_SRC: &'static str = "
#version 150
in vec2 position;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}";

static FS_SRC: &'static str = "
#version 150
out vec4 out_color;
void main() {
    out_color = vec4(1.0, 1.0, 1.0, 1.0);
}";

static GS_SRC: &'static str = "
#version 150
layout (triangles) in;
layout (triangle_strip, max_vertices = 3) out;
void main() {
    for (int i = 0; i < 3; i++) 
    {
            //gl_Position = gl_in[i].gl_Position;
            //EmitVertex();
            //continue;

        //gl_Position = gl_in[i].gl_Position;
        vec4 position = gl_in[i].gl_Position;
        gl_Position = vec4(position.x - 0.05, position.y - 0.1, position.z, position.w);
        EmitVertex();
        gl_Position = vec4(position.x + 0.05, position.y - 0.1, position.z, position.w);
        EmitVertex();
        gl_Position = vec4(position.x, position.y + 0.1, position.z, position.w);
        EmitVertex();
    }
    EndPrimitive();
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

fn do_loop() {
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

        static VERTEX_DATA: [GLfloat; 6] = [0.0, 0.5, 0.5, -0.5, -0.5, -0.5];
        //static VERTEX_DATA: [GLfloat; 9] = [-1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 0.0, 1.0, 0.0];
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            mem::transmute(&VERTEX_DATA[0]),
            gl::STATIC_DRAW,
        );
        gl::UseProgram(program);
        gl::BindFragDataLocation(program, 0, CString::new("out_color").unwrap().as_ptr());

        // Specify the layout of the vertex data
        let pos_attr = gl::GetAttribLocation(program, CString::new("position").unwrap().as_ptr());
        gl::EnableVertexAttribArray(pos_attr as GLuint);
        gl::VertexAttribPointer(
            pos_attr as GLuint,
            2,
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
                gl::ClearColor(0.0, 0.0, 1.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
                let version = gl::GetString(gl::VERSION);
                println!("redraw yo! {:?}", CStr::from_ptr(version as *const i8));
                // Use shader program
                gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 3);

                windowed_context.swap_buffers().unwrap();
            },
            _ => *control_flow = ControlFlow::Wait,
        },
        _ => *control_flow = ControlFlow::Wait,
    });
}

fn main() {
    let matches = App::new("Tracer")
        .version("1.0")
        .arg(
            Arg::with_name("chrome_trace")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let file = trace::google_trace::load_file(
        matches
            .value_of("chrome_trace")
            .expect("Must specify chrome_trace"),
    )
    .unwrap();

    println!(
        "Hello, world! {:?}",
        (
            file.threads.len(),
            file.async_events.len(),
            file.event_types.len()
        )
    );
    for event_type in file.event_types {
        let event_type: &TraceEventType = &event_type;
        println!(
            "event {}: {}",
            event_type.name,
            event_type.count.load(Ordering::SeqCst)
        );
    }

    do_loop();
}

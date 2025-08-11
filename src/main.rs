use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

/**
 * Y-axis rotation matrix
 */
fn rotation_matrix_y(angle: f32) -> [f32; 16] {
    let (s, c) = angle.sin_cos();
    [
        c, 0.0, s, 0.0, 0.0, 1.0, 0.0, 0.0, -s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

// Vertex shader
const VERT: &str = r#"
attribute vec3 position;
attribute vec3 color;
uniform mat4 modelViewMatrix;
varying vec3 vColor;
void main() {
    gl_Position = modelViewMatrix * vec4(position, 1.0);
    vColor = color;
}
"#;

// Fragment shader
const FRAG: &str = r#"
precision mediump float;
varying vec3 vColor;
void main() {
    gl_FragColor = vec4(vColor, 1.0);
}
"#;

// Entry point
fn main() {
    dioxus::launch(app);
}

// WebGL initialization flag
static WEBGL_INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn app() -> Element {
    let mut canvas_mounted = use_signal(|| false);

    use_effect(move || {
        if !canvas_mounted() {
            return;
        }

        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(50).await;

            // Ensure it runs only once (static control)
            static INIT_ONCE: std::sync::Once = std::sync::Once::new();
            let mut should_init = false;

            INIT_ONCE.call_once(|| {
                should_init = true;
            });

            if !should_init {
                web_sys::console::log_1(&"WebGL initialization already completed".into());
                return;
            }

            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let canvas = document
                .get_element_by_id("webgl-canvas")
                .unwrap()
                .dyn_into::<HtmlCanvasElement>()
                .unwrap();

            web_sys::console::log_1(&"Initializing WebGL (single time)...".into());

            let gl: WebGl2RenderingContext = canvas
                .get_context("webgl2")
                .unwrap()
                .unwrap()
                .dyn_into::<WebGl2RenderingContext>()
                .unwrap();

            // Initial WebGL setup
            canvas.set_width(480);
            canvas.set_height(480);
            gl.viewport(0, 0, 480, 480);
            // Keep depth test disabled (ensure the cube is always visible)
            gl.disable(WebGl2RenderingContext::DEPTH_TEST);
            gl.disable(WebGl2RenderingContext::CULL_FACE);

            web_sys::console::log_1(&"WebGL context configured".into());

            // Create and compile shaders
            let vert_shader = gl
                .create_shader(WebGl2RenderingContext::VERTEX_SHADER)
                .unwrap();
            gl.shader_source(&vert_shader, VERT);
            gl.compile_shader(&vert_shader);

            // Check vertex shader compilation result
            if !gl
                .get_shader_parameter(&vert_shader, WebGl2RenderingContext::COMPILE_STATUS)
                .as_bool()
                .unwrap_or(false)
            {
                let log = gl.get_shader_info_log(&vert_shader).unwrap_or_default();
                web_sys::console::error_1(
                    &format!("Vertex shader compilation error: {}", log).into(),
                );
                return;
            }

            let frag_shader = gl
                .create_shader(WebGl2RenderingContext::FRAGMENT_SHADER)
                .unwrap();
            gl.shader_source(&frag_shader, FRAG);
            gl.compile_shader(&frag_shader);

            // Check fragment shader compilation result
            if !gl
                .get_shader_parameter(&frag_shader, WebGl2RenderingContext::COMPILE_STATUS)
                .as_bool()
                .unwrap_or(false)
            {
                let log = gl.get_shader_info_log(&frag_shader).unwrap_or_default();
                web_sys::console::error_1(
                    &format!("Fragment shader compilation error: {}", log).into(),
                );
                return;
            }

            // Create and link program
            let program = gl.create_program().unwrap();
            gl.attach_shader(&program, &vert_shader);
            gl.attach_shader(&program, &frag_shader);
            gl.link_program(&program);

            // Check program link result
            if !gl
                .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
                .as_bool()
                .unwrap_or(false)
            {
                let log = gl.get_program_info_log(&program).unwrap_or_default();
                web_sys::console::error_1(&format!("Program linking error: {}", log).into());
                return;
            }

            gl.use_program(Some(&program));

            web_sys::console::log_1(&"Shaders compiled and program linked".into());

            // Cube vertex data (moderate size to ensure visibility)
            let vertices: [f32; 24] = [
                // Four front-face vertices (Z=0.2)
                -0.4, -0.4, 0.2, 0.4, -0.4, 0.2, 0.4, 0.4, 0.2, -0.4, 0.4, 0.2,
                // Four back-face vertices (Z=-0.2)
                -0.4, -0.4, -0.2, 0.4, -0.4, -0.2, 0.4, 0.4, -0.2, -0.4, 0.4, -0.2,
            ];

            let colors: [f32; 24] = [
                // Front face colors
                1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0,
                // Back face colors
                1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.5, 0.5,
            ];

            let indices: [u16; 36] = [
                // Front
                0, 1, 2, 2, 3, 0, // Back (clockwise)
                4, 6, 5, 6, 4, 7, // Left
                4, 0, 3, 3, 7, 4, // Right
                1, 5, 6, 6, 2, 1, // Top
                3, 2, 6, 6, 7, 3, // Bottom
                4, 5, 1, 1, 0, 4,
            ];

            // Vertex buffer
            let pos_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&pos_buffer));
            unsafe {
                let vert_array = js_sys::Float32Array::view(&vertices);
                gl.buffer_data_with_array_buffer_view(
                    WebGl2RenderingContext::ARRAY_BUFFER,
                    &vert_array,
                    WebGl2RenderingContext::STATIC_DRAW,
                );
            }

            // Color buffer
            let color_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&color_buffer));
            unsafe {
                let color_array = js_sys::Float32Array::view(&colors);
                gl.buffer_data_with_array_buffer_view(
                    WebGl2RenderingContext::ARRAY_BUFFER,
                    &color_array,
                    WebGl2RenderingContext::STATIC_DRAW,
                );
            }

            // Index buffer
            let index_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(
                WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
                Some(&index_buffer),
            );
            unsafe {
                let index_array = js_sys::Uint16Array::view(&indices);
                gl.buffer_data_with_array_buffer_view(
                    WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
                    &index_array,
                    WebGl2RenderingContext::STATIC_DRAW,
                );
            }

            // Attribute setup
            let pos_loc = gl.get_attrib_location(&program, "position");
            let color_loc = gl.get_attrib_location(&program, "color");

            web_sys::console::log_1(
                &format!(
                    "Position attribute location: {}, Color attribute location: {}",
                    pos_loc, color_loc
                )
                .into(),
            );

            if pos_loc < 0 || color_loc < 0 {
                web_sys::console::error_1(&"Failed to get attribute locations".into());
                return;
            }

            let pos_loc = pos_loc as u32;
            let color_loc = color_loc as u32;

            gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&pos_buffer));
            gl.enable_vertex_attrib_array(pos_loc);
            gl.vertex_attrib_pointer_with_i32(
                pos_loc,
                3,
                WebGl2RenderingContext::FLOAT,
                false,
                0,
                0,
            );

            gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&color_buffer));
            gl.enable_vertex_attrib_array(color_loc);
            gl.vertex_attrib_pointer_with_i32(
                color_loc,
                3,
                WebGl2RenderingContext::FLOAT,
                false,
                0,
                0,
            );

            web_sys::console::log_1(&"Buffers and attributes configured".into());

            // Animation loop
            let animation_loop = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
            let animation_loop_clone = animation_loop.clone();
            let angle = Rc::new(RefCell::new(0.0f32));

            *animation_loop_clone.borrow_mut() = Some(Closure::wrap(Box::new({
                let angle = angle.clone();
                let animation_loop = animation_loop.clone();
                let frame_count = Rc::new(RefCell::new(0u32));
                move || {
                    let mut current_angle = *angle.borrow();
                    let mut count = *frame_count.borrow();
                    count += 1;
                    *frame_count.borrow_mut() = count;

                    if count % 60 == 0 {
                        web_sys::console::log_1(
                            &format!("Rendering frame {}, angle: {:.2}", count, current_angle)
                                .into(),
                        );
                    }

                    // Clear background (do not use depth buffer)
                    gl.clear_color(0.1, 0.1, 0.1, 1.0);
                    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

                    // Simple rotation matrix only (reliable setting)
                    let model = rotation_matrix_y(current_angle);

                    // Pass matrix to the uniform variable
                    let loc = gl.get_uniform_location(&program, "modelViewMatrix");
                    gl.uniform_matrix4fv_with_f32_array(loc.as_ref(), false, &model);

                    // Draw
                    gl.bind_buffer(
                        WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER,
                        Some(&index_buffer),
                    );
                    gl.draw_elements_with_i32(
                        WebGl2RenderingContext::TRIANGLES,
                        indices.len() as i32,
                        WebGl2RenderingContext::UNSIGNED_SHORT,
                        0,
                    );

                    // Check WebGL errors
                    let error = gl.get_error();
                    if error != WebGl2RenderingContext::NO_ERROR {
                        web_sys::console::error_1(&format!("WebGL error: {}", error).into());
                    }

                    // Update angle
                    current_angle += 0.02;
                    *angle.borrow_mut() = current_angle;

                    // Next frame
                    web_sys::window()
                        .unwrap()
                        .request_animation_frame(
                            animation_loop
                                .borrow()
                                .as_ref()
                                .unwrap()
                                .as_ref()
                                .unchecked_ref(),
                        )
                        .unwrap();
                }
            })
                as Box<dyn FnMut()>));

            // Start animation
            web_sys::window()
                .unwrap()
                .request_animation_frame(
                    animation_loop_clone
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unchecked_ref(),
                )
                .unwrap();

            web_sys::console::log_1(&"Animation started successfully!".into());
        });
    });

    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; height: 100vh; background: #f0f0f0;",
            canvas {
                id: "webgl-canvas",
                width: "480",
                height: "480",
                style: "border: 2px solid #333; background: #222;",
                onmounted: move |_| {
                    canvas_mounted.set(true);
                }
            }
        }
    }
}

#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::{io::Cursor, rc::Rc};

use audio::AudioPlayer;
use audioviz::spectrum::{config::StreamConfig, stream::Stream, Frequency};
use pixels::{PixelsBuilder, SurfaceTexture};
use symphonia::core::audio::Signal;
use tiny_skia::{Paint, Pixmap, Point, Rect, Transform};
use utils::pick_file;
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

mod audio;
mod utils;

const WIDTH: u32 = 1000;
const HEIGHT: u32 = 500;

pub async fn run() {
    let event_loop = EventLoop::new();
    let window = {
        let inner_size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("synthviz")
            .with_inner_size(inner_size)
            .with_min_inner_size(inner_size)
            .with_visible(false)
            .build(&event_loop)
            .expect("failed to build window")
    };

    let window = Rc::new(window);
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowExtWebSys;

        // Retrieve current width and height dimensions of browser client window
        let get_window_size = || {
            let client_window = web_sys::window().unwrap();
            LogicalSize::new(
                client_window.inner_width().unwrap().as_f64().unwrap(),
                client_window.inner_height().unwrap().as_f64().unwrap(),
            )
        };

        let window = Rc::clone(&window);

        // Initialize winit window with current dimensions of browser client
        window.set_inner_size(get_window_size());

        let client_window = web_sys::window().unwrap();

        // Attach winit canvas to body element
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                let canvas = window.canvas();
                canvas
                    .set_attribute("oncontextmenu", "return false")
                    .unwrap();
                body.append_child(&web_sys::Element::from(canvas)).ok()
            })
            .expect("couldn't append canvas to document body");

        // Listen for resize event on browser client. Adjust winit window dimensions
        // on event trigger
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let size = get_window_size();
            window.set_inner_size(size)
        }) as Box<dyn FnMut(_)>);
        client_window
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    let mut input = WinitInputHelper::new();

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.as_ref());
        PixelsBuilder::new(WIDTH, HEIGHT, surface_texture)
            .build_async()
            .await
            .expect("failed to create pixels")
    };
    let mut drawing = Pixmap::new(WIDTH, HEIGHT).unwrap();

    let mut file = pick_file().await;
    while file.is_none() {
        file = pick_file().await;
    }
    let file = file.unwrap();
    let buf = file.read().await;
    let cur = Cursor::new(buf);

    let mut player = AudioPlayer::new();
    player.load(cur).expect("couldn't load audio");

    let mut stream = Stream::new(StreamConfig::default());

    window.set_visible(true);
    window.focus_window();
    event_loop.run(move |event, _, control_flow| {
        if let Some(buf) = player.decode() {
            stream.push_data(buf.chan(0).to_vec());
            stream.update();
        }

        if let Event::RedrawRequested(_) = event {
            fn draw_line(
                drawing: &mut Pixmap,
                x1: f32,
                y1: f32,
                x2: f32,
                y2: f32,
                thickness: f32,
                color: [u8; 4],
            ) {
                let mut paint = Paint::default();
                paint.set_color_rgba8(color[0], color[1], color[2], color[3]);

                let rect = Rect::from_points(&[
                    Point::from_xy(x1 - thickness, y1),
                    Point::from_xy(x2 + thickness, y2),
                ])
                .unwrap();

                drawing.fill_rect(rect, &paint, Transform::identity(), None);
            }

            let frequencies = stream.get_frequencies();
            let frequencies: Vec<Frequency> = if frequencies.len() >= 2 {
                let mut buf: Vec<Frequency> = Vec::new();
                // left
                let mut left = frequencies[0].clone();
                left.reverse();
                buf.append(&mut left);
                // right
                buf.append(&mut frequencies[1].clone());
                buf
            } else {
                if frequencies.len() == 1 {
                    frequencies[0].clone()
                } else {
                    Vec::new()
                }
            };

            let height = HEIGHT as f32;
            let width = WIDTH as f32;

            let mut freqs = frequencies.iter().peekable();
            let mut x = 0.5;

            loop {
                // determines positions of line
                let f1: &Frequency = match freqs.next() {
                    Some(d) => d,
                    None => break,
                };
                let f2: &Frequency = match freqs.peek() {
                    Some(d) => *d,
                    None => break,
                };
                let y1: f32 = height - (f1.volume * height);
                let y2: f32 = height - (f2.volume * height);

                let x1: f32 = (x / frequencies.len() as f32) * width;
                let x2: f32 = ((x + 1.0) / frequencies.len() as f32) * width;

                draw_line(&mut drawing, x1, y1, x2, y2, 4.0, [0xFF, 0xFF, 0xFF, 0xFF]);

                x += 1.0;
            }

            pixels.frame_mut().copy_from_slice(drawing.data());
            pixels.render().expect("failed to render");
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if input.mouse_released(1) {}

            if let Some(size) = input.window_resized() {
                pixels
                    .resize_surface(size.width, size.height)
                    .expect("couldn't resize surface");
            }

            window.request_redraw();
        }
    });
}

fn main() {
    utils::set_panic_hook();
    utils::init_log();

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(run());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run());
    }
}

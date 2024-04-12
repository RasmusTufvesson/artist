use std::{env::args, thread::sleep, time::Duration};
use enigo::{Key, KeyboardControllable, MouseButton, MouseControllable};
use image::DynamicImage;

const SLEEP_TIME: Duration = Duration::from_secs(3);
const DOT_WIDTH_FLOAT: f32 = 5.;
const DOT_WIDTH: i32 = 5;

fn main() {
    let image_path = args().nth(1).expect("Specify image path");
    let img = image::open(image_path).expect("Could not open image");
    let mut artist = Artist::new(img);
    artist.paint();
}

struct Artist {
    enigo: enigo::Enigo,
    img: DynamicImage,
    left: i32,
    top: i32,
    black_x: i32,
    black_y: i32,
    width: i32,
    height: i32,
}

impl Artist {
    fn new(img: DynamicImage) -> Self {
        let mut enigo = enigo::Enigo::new();
        sleep(SLEEP_TIME);
        let (left, top) = enigo.mouse_location();
        enigo.mouse_click(MouseButton::Left);
        sleep(SLEEP_TIME);
        let (right, bottom) = enigo.mouse_location();
        enigo.mouse_click(MouseButton::Left);
        sleep(SLEEP_TIME);
        let (black_x, black_y) = enigo.mouse_location();
        enigo.key_down(Key::Control);
        enigo.key_down(Key::A);
        enigo.key_up(Key::Control);
        enigo.key_up(Key::A);
        enigo.key_click(Key::Delete);
        enigo.key_click(Key::Alt);
        enigo.key_click(Key::B);
        enigo.key_click(Key::Alt);
        enigo.key_click(Key::S);
        enigo.key_click(Key::Z);
        enigo.key_click(Key::UpArrow);
        enigo.key_click(Key::Return);
        let painting_width = right - left;
        let painting_height = bottom - top;
        let horizontal_dots = (painting_width as f32 / DOT_WIDTH_FLOAT).ceil() as i32;
        let vertical_dots = (painting_height as f32 / DOT_WIDTH_FLOAT).ceil() as i32;
        Self { enigo, img, left, top, black_x, black_y, width: horizontal_dots, height: vertical_dots }
    }

    fn paint(&mut self) {
        self.draw_dot(0, 0);
        for x in 0..=self.width {
            for y in 0..=self.height {
                let (canvas_x, canvas_y) = (x * DOT_WIDTH, y * DOT_WIDTH);
                self.draw_dot(canvas_x, canvas_y);
            }
        }
    }

    fn click(&mut self, x: i32, y: i32) {
        self.enigo.mouse_move_to(x, y);
        self.enigo.mouse_click(MouseButton::Left);
    }

    fn draw_dot(&mut self, x: i32, y: i32) {
        self.click(self.left + x, self.top + y);
    }
}
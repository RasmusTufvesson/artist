use std::{env::args, thread::sleep, time::Duration};
use enigo::{Key, KeyboardControllable, MouseButton, MouseControllable};

const SLEEP_TIME: Duration = Duration::from_secs(3);
const DOT_WIDTH: i32 = 7;

fn main() {
    let image_path = args().nth(1).expect("Specify image path");
    let img = image::open(image_path).expect("Could not open image");
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
}

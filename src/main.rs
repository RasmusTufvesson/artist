use std::env::args;

use image::GenericImageView;

fn main() {
    if let Some(image_path) = args().nth(1) {
        let img = image::open(image_path).unwrap();
        println!("{:?}", img.get_pixel(0, 0));
    }
}

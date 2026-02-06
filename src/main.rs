use std::error::Error;
use std::fs::File;
use image::codecs::png::PngEncoder;
use image::{ColorType, ExtendedColorType, ImageEncoder};
use num::{Complex, pow};
use num::integer::{div_ceil, div_floor};
use rayon::prelude::*;

type EscapeLimit = u16;

fn calculate_escape_time(c: Complex<f32>, limit: EscapeLimit) -> Option<EscapeLimit>{
    let mut z = Complex::<f32> {re: 0.0, im: 0.0};
    for i in 0..limit{
        let norm_sqr = z.norm_sqr();
        if norm_sqr > 4.0{
            return Some(i);
        } else if (i > 0) && (norm_sqr <= 10e-6){
            return None;
        } else {
            z = z * z + c;
        }
    }
    None
}


fn make_lerp(input: &(f32, f32), output: &(f32, f32)) -> impl Fn(f32) -> f32 {
    let a = (output.1 - output.0) / (input.1 - input.0);
    let b =  output.0 - (input.0 * a);
    move |x| x*a + b
}


#[derive(Clone)]
struct Resolution{
    width: u32, height: u32
}
struct Range<T>{
    min: T,
    max: T,
}
struct Rect<T>{
    x: Range<T>,
    y: Range<T>,
}

fn make_calculations(resolution: Resolution, rect: Rect<f32>, limit: EscapeLimit)
                     -> impl Iterator<Item = ((u32, u32), Option<EscapeLimit>)> {

    let Rect{x: Range{min: x_min, max: x_max} ,y: Range{min: y_min, max: y_max}} = rect;

    let values =
        (0..resolution.width.clone())
        .map(
            move |x|
                (0..resolution.height.clone())
                .map(move |y| (x, y))
        )
        .flatten()
        .map({
            let x_lerp = make_lerp(&(0_f32, resolution.width as f32), &(x_min, x_max));
            let y_lerp = make_lerp(&( resolution.height as f32, 0_f32), &(y_min, y_max));

            move |(x, y)| {
                let x_c = x_lerp(x as f32);
                let y_c = y_lerp(y as f32);
                let c = Complex::<f32>{re: x_c, im: y_c};
                ((x, y), calculate_escape_time(c, limit))
            }
            .clone()
        });
    values
}


struct Image{
    resolution: Resolution,
    data:       Box<[u8]>,
}

impl Image{
    fn new(res: &Resolution) -> Self{
        let data = vec![0; (res.width as usize) * (res.height as usize)];

        Image{
            resolution: res.clone(),
            data: data.into_boxed_slice()
        }
    }
}


fn write_data(
        img: &mut Image,
        data: impl Iterator<Item=((u32, u32), Option<EscapeLimit>)>,
        escape_limit: EscapeLimit,
    ) {

    let const_mul =  255_f32 / escape_limit as f32;
    for (position, value) in data {
        let index = (position.0 + position.1 * img.resolution.width) as usize;

        if let Some(pixel) = img.data.get_mut(index) {
            *pixel = match value {
                None => { 0 }
                Some(val) => { 255 - (val as f32 * const_mul) as u8 }
            }
        }
    }

}

fn save_image(img: &Image, file_name: &str) -> Result<(), Box<dyn Error>>{
    let output = File::create(format!("{file_name}.png"))?;
    let encoder = PngEncoder::new(output);

    encoder
    .write_image(
        &img.data,
        img.resolution.width,
        img.resolution.height,
        ColorType::L8.into(),
    )
    .expect("Error while trying to save this shit");

    Ok(())
}

fn main(){
    const RESOLUTION: Resolution = Resolution{width: 1024*2*2*2, height: 1024*2*2*2};
    const LIMIT: EscapeLimit = 256;
    let dest = "atlas/";
    std::fs::create_dir_all(dest).unwrap();

    let rect_lin_num = 128;

    let x_rect_lerp = make_lerp(&(0.0, rect_lin_num as f32), &(-2.0, 1.0) );
    let y_rect_lerp = make_lerp(&(0.0, rect_lin_num as f32), &(-1.5, 1.5) );

    let atlas_squares =
        (0..rect_lin_num)
        .map(move |x| (0..rect_lin_num).map(move|y|(x, y)))
        .flatten()
        .par_bridge()
        .map(|(x_i, y_i)|{
                let rect = {
                       let x_range = Range::<f32>{min: x_rect_lerp(x_i as f32) , max: x_rect_lerp((x_i + 1) as
                       f32)};
                       let y_range = Range::<f32>{min: y_rect_lerp(y_i as f32) , max: y_rect_lerp((y_i + 1) as
                       f32)};

                       Rect::<f32>{x: x_range,y: y_range}
                };

                let string_end = format!("[{:02.3},{:02.3}]_[{:02.3},{:02.3}]", &rect.x.min, &rect.x
                .max, &rect.y.min, &rect.y.max);

                let file_name = format!("atlas/mandelbrot_{}", &string_end);
                let mut image = Image::new(&RESOLUTION);
                let calculations = make_calculations(RESOLUTION, rect, LIMIT);
                println!("Starting calculations for {}", &string_end);
                write_data(&mut image, calculations, LIMIT);

                if (image.data.iter().max().unwrap() - image.data.iter().min().unwrap()) > 20 {
                    println!("Writing file for {}", &string_end );
                    save_image(&image, &file_name).unwrap();
                } else {
                    println!("Skipping {}", &string_end);

                }
            }
        );


    let _: Vec<()> = atlas_squares.collect();
    println!("all finished")
}
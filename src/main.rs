use image::{self, DynamicImage, GenericImageView, ImageReader};
use rodio::{Decoder, OutputStream, Source};
use std::{
    env,
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, Write},
    path::Path,
    thread::sleep,
    time::{Duration, Instant},
};

const VIDEO_FRAME_RESOLUTION: (u32, u32) = (480, 360);

fn print_usage() {
    println!("Usage: badapple-terminal <run|compile>");
}

fn print_unknown_command(command: &str) {
    println!("Unknown command: {}", command);
    print_usage()
}

fn get_pixel_color(x: u32, y: u32, image: &DynamicImage, pixel_size: (f64, f64)) -> u32 {
    let mut color = 0.0;
    let step_x = pixel_size.0.ceil() as u32;
    let step_y = pixel_size.1.ceil() as u32;

    for i in 0..step_x {
        for j in 0..step_y {
            let pixel_color = image.get_pixel(x + i, y + j);
            color += pixel_color.0[0] as f64;
        }
    }

    color /= (step_y * step_y) as f64;
    color.round() as u32
}

fn build_frames(resolution: (u32, u32), ch: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Building frames at resolution {}x{}",
        resolution.0, resolution.1
    );

    let pixel_size = (
        VIDEO_FRAME_RESOLUTION.0 as f64 / resolution.0 as f64,
        VIDEO_FRAME_RESOLUTION.1 as f64 / resolution.1 as f64,
    );

    let files: Vec<_> = fs::read_dir("./assets/frames")?
        .filter_map(|entry| entry.ok())
        .collect();
    let num_files = files.len();

    let step_y = pixel_size.1.ceil() as u32;
    let end_y = VIDEO_FRAME_RESOLUTION.1 - step_y;
    let step_x = pixel_size.0.ceil() as u32;
    let end_x = VIDEO_FRAME_RESOLUTION.0 - step_x;

    // TODO: This totally doesn't work as intended. Because of the way I'm rounding,
    // you can't actually build the frames at an arbitrary resolution. It always
    // snaps to the closest whole pixel. I can't be bothered to fix this, though.
    for (i, file) in files.iter().enumerate() {
        print!("\rConverting frame {}/{}", i + 1, num_files);

        let image = ImageReader::open(file.path())?.decode()?;
        let mut frame_output = String::new();

        for y in (0..=end_y).step_by(step_y as usize) {
            frame_output.push_str("\r");

            for x in (0..=end_x).step_by(step_x as usize) {
                let color = get_pixel_color(x, y, &image, pixel_size);
                let char = if color >= 125 { " " } else { ch };
                frame_output.push_str(char);
            }

            frame_output.push_str("\x1b[1B");
        }

        let file_path = file.path();
        let file_name = file_path.file_stem().unwrap();
        let output_path =
            Path::new("./assets/output").join(format!("{}", file_name.to_string_lossy()));
        let mut output_file = File::create(output_path)?;
        output_file.write_all(frame_output.as_bytes())?;
    }

    println!();
    println!("Done");

    Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut files: Vec<_> = fs::read_dir("./assets/output")?
        .filter_map(|entry| entry.ok())
        .collect();

    files.sort_by_key(|entry| {
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        let numeric_part = file_name_str.split('.').next().unwrap_or("0");

        numeric_part.parse::<u32>().unwrap_or(0)
    });

    let frames: Vec<_> = files
        .iter()
        .map(|file| {
            let mut buffer = Vec::new();
            let mut f = File::open(file.path())?;
            f.read_to_end(&mut buffer)?;
            Ok(buffer)
        })
        .collect::<Result<Vec<Vec<u8>>, io::Error>>()?;

    let mut buffer = BufWriter::new(io::stdout());

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let file = BufReader::new(File::open("./assets/music.mp3").unwrap());
    let source = Decoder::new(file).unwrap();
    let _ = stream_handle.play_raw(source.convert_samples());

    let interval = Duration::from_secs_f64(1.0 / 30.0);
    let mut next_time = Instant::now() + interval;

    buffer.write(b"\x1b[2J").unwrap();

    for frame in frames {
        buffer.write(b"\x1b[0;0H").unwrap();

        buffer.write(&frame).unwrap();
        buffer.flush().unwrap();

        sleep(next_time - Instant::now());
        next_time += interval;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    if command == "run" {
        if args.len() >= 4 {
            let resolution = (
                args[2].parse::<u32>().unwrap(),
                args[3].parse::<u32>().unwrap(),
            );
            let ch = if args.len() == 5 { &args[4] } else { "@" };
            build_frames(resolution, ch).unwrap();
        }
        run()
    } else if command == "build" {
        let resolution = if args.len() >= 4 {
            (
                args[2].parse::<u32>().unwrap(),
                args[3].parse::<u32>().unwrap(),
            )
        } else {
            VIDEO_FRAME_RESOLUTION
        };
        let ch = if args.len() == 5 { &args[4] } else { "@" };
        build_frames(resolution, ch)
    } else {
        print_unknown_command(&args[1]);
        Ok(())
    }
}

use flate2::read::GzDecoder;
use orb_slam3_rs::{OrbSlam, SensorType};
use std::fs::File;
use std::io::{Write, copy};
use std::path::Path;
use std::time::{Duration, Instant};

// This download do not works. Please directly unzip 3rdparty/ORB_SLAM3/Vocabulary.
fn ensure_vocabulary(path: &str) {
    if Path::new(path).exists() {
        return;
    }

    let gz_path = "examples/ORBvoc.txt.gz";
    std::fs::create_dir_all("examples").ok();

    println!("[INFO] Vocabulary not found. Downloading (this may take a minute)...");

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()
        .expect("Failed to create HTTP client");

    let url = "https://github.com/UZ-SLAMLab/ORB_SLAM3/raw/refs/heads/master/Vocabulary/ORBvoc.txt.tar.gz";
    let mut response = client
        .get(url)
        .send()
        .expect("Failed to download vocabulary");

    if !response.status().is_success() {
        panic!(
            "Failed to download: Server returned status {}",
            response.status()
        );
    }

    let mut gz_file = File::create(gz_path).expect("Failed to create local gz file");
    copy(&mut response, &mut gz_file).expect("Failed to write vocabulary to disk");
    gz_file.sync_all().unwrap();

    println!("[INFO] Decompressing vocabulary...");

    let gz_file_read = File::open(gz_path).expect("Failed to open downloaded gz file");
    let mut decoder = GzDecoder::new(gz_file_read);
    let mut out_file = File::create(path).expect("Failed to create ORBvoc.txt");

    copy(&mut decoder, &mut out_file)
        .expect("Failed to decompress: The file downloaded was not a valid Gzip archive.");

    out_file.sync_all().expect("Failed to sync output file");

    std::fs::remove_file(gz_path).ok();
    println!("[INFO] Vocabulary ready at {path}.");
}

fn main() {
    let voc_path = "examples/ORBvoc.txt";
    std::fs::create_dir_all("examples").ok();
    ensure_vocabulary(voc_path);

    println!("[INFO] Initializing ORB-SLAM3...");
    let mut slam = OrbSlam::new(voc_path, "examples/drone.yaml", SensorType::Monocular);

    if !slam.open_video("examples/drone_video.mp4") {
        panic!("Please put a video in examples/drone_video.mp4");
    }

    let mut frame_id = 0;
    let mut last_log_time = Instant::now();
    let log_interval = Duration::from_secs(30);
    let mut total_distance: f32 = 0.0;
    let mut last_pose: Option<(f32, f32, f32)> = None;
    let start_time = Instant::now();

    loop {
        let frame_start = Instant::now();
        let pose = slam.process_next_video_frame();

        if pose.is_eof {
            println!("\n[INFO] Video ended.");
            break;
        }

        if pose.is_valid {
            if let Some((lx, ly, lz)) = last_pose {
                let dx = pose.tx - lx;
                let dy = pose.ty - ly;
                let dz = pose.tz - lz;

                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                total_distance += dist;
            }

            last_pose = Some((pose.tx, pose.ty, pose.tz));

            if last_log_time.elapsed() >= log_interval {
                println!(
                    "\n--- [LOG 30s] Position: X={:.2}, Y={:.2}, Z={:.2} | Total dist.: {:.2} ---",
                    pose.tx, pose.ty, pose.tz, total_distance
                );
                last_log_time = Instant::now();
            }

            let points = slam.get_tracked_points();
            let elapsed = frame_start.elapsed().as_secs_f32();
            let fps = if elapsed > 0.0 { 1.0 / elapsed } else { 0.0 };

            print!(
                "\r[FRAME {:04}] OK | XYZ: {:>5.1}, {:>5.1}, {:>5.1} | Coord.: {:>4} | {:.1} FPS",
                frame_id,
                pose.tx,
                pose.ty,
                pose.tz,
                points.len(),
                fps
            );
        } else {
            print!("\r[FRAME {:04}] Finding keypoints...", frame_id);
        }

        std::io::stdout().flush().unwrap();
        frame_id += 1;
    }

    println!("\nTotal distance: {total_distance:.2} units");
    println!("Total time: {:?}", start_time.elapsed());
}

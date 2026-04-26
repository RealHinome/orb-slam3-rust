#![allow(unused_unsafe)]
#![allow(unsafe_op_in_unsafe_fn)]

use autocxx::prelude::*;

include_cpp! {
    #include "slam_wrapper.h"
    safety!(unsafe_ffi)

    generate_pod!("Point3D")
    generate_pod!("Pose")
    generate!("SlamWrapper")
}

pub use ffi::Point3D;
pub use ffi::Pose;

pub enum SensorType {
    Monocular = 0,
    Stereo = 1,
    Rgbd = 2,
    ImuMonocular = 3,
}

pub struct OrbSlam {
    inner: cxx::UniquePtr<ffi::SlamWrapper>,
}

impl OrbSlam {
    /// Initialize the SLAM System.
    ///
    /// # Arguments
    /// * `voc_file` - Path to the vocabulary file (e.g., ORBvoc.txt)
    /// * `settings_file` - Path to the camera calibration/settings YAML
    /// * `sensor` - Type of sensor used
    pub fn new(voc_file: &str, settings_file: &str, sensor: SensorType) -> Self {
        cxx::let_cxx_string!(voc_cxx = voc_file);
        cxx::let_cxx_string!(set_cxx = settings_file);

        let inner =
            ffi::SlamWrapper::new(&voc_cxx, &set_cxx, (sensor as i32).into()).within_unique_ptr();

        Self { inner }
    }

    /// Opens a video file or stream for processing.
    ///
    /// # Arguments
    /// * `filepath` - A string slice containing the path to the video file or a
    ///   network stream URL (e.g., "rtsp://...").
    pub fn open_video(&mut self, filepath: &str) -> bool {
        cxx::let_cxx_string!(path_cxx = filepath);
        let mut pinned = self.inner.pin_mut();
        pinned.as_mut().open_video(&path_cxx)
    }

    /// Reads the next frame from the opened video source and processes it
    /// through the SLAM system.
    ///
    /// # Returns
    /// * `Pose` - A struct containing:
    ///     * `is_valid`: `true` if the SLAM system successfully tracked the camera position.
    ///     * `is_eof`: `true` if the end of the video was reached or an error occurred.
    ///     * `tx, ty, tz, qx, qy, qz, qw`: The 3D translation and rotation (quaternion).
    pub fn process_next_video_frame(&mut self) -> Pose {
        let mut pinned = self.inner.pin_mut();
        pinned.as_mut().process_next_video_frame()
    }

    /// Retrieve 3D coordinates of currently tracked features
    pub fn get_tracked_points(&self) -> Vec<Point3D> {
        let points_cxx = self.inner.get_tracked_map_points();
        let mut rust_points = Vec::with_capacity(points_cxx.len());
        for p in points_cxx.iter() {
            rust_points.push(Point3D {
                x: p.x,
                y: p.y,
                z: p.z,
            });
        }
        rust_points
    }

    /// Shutdown the SLAM system safely.
    pub fn shutdown(&mut self) {
        let mut pinned = self.inner.pin_mut();
        pinned.as_mut().shutdown();
    }
}

impl Drop for OrbSlam {
    fn drop(&mut self) {
        self.shutdown();
    }
}

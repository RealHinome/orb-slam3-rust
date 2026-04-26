#pragma once

#include <string>
#include <vector>

struct Point3D {
  float x, y, z;
};

struct Pose {
  float tx, ty, tz;
  float qx, qy, qz, qw;
  bool is_valid;
  bool is_eof;
};

namespace ORB_SLAM3 {
class System;
}

class SlamWrapper {
 public:
  SlamWrapper(const std::string& voc_file, const std::string& settings_file,
              int sensor_type);
  ~SlamWrapper();

  // Méthodes pour la vidéo
  bool open_video(const std::string& filepath);
  Pose process_next_video_frame();

  std::vector<Point3D> get_tracked_map_points() const;
  void shutdown();

 private:
  ORB_SLAM3::System* slam_system_;
  void* video_capture_;  // Hidden pointer for cv::VideoCapture
};

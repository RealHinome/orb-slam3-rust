#include "slam_wrapper.h"

#include <opencv2/opencv.hpp>

#include "System.h"

SlamWrapper::SlamWrapper(const std::string& voc_file,
                         const std::string& settings_file, int sensor_type) {
  ORB_SLAM3::System::eSensor sensor =
      static_cast<ORB_SLAM3::System::eSensor>(sensor_type);
  slam_system_ = new ORB_SLAM3::System(voc_file, settings_file, sensor, false);
  video_capture_ = nullptr;
}

SlamWrapper::~SlamWrapper() {
  shutdown();
  if (video_capture_) {
    delete static_cast<cv::VideoCapture*>(video_capture_);
    video_capture_ = nullptr;
  }
  if (slam_system_) {
    delete slam_system_;
    slam_system_ = nullptr;
  }
}

bool SlamWrapper::open_video(const std::string& filepath) {
  cv::VideoCapture* cap = new cv::VideoCapture(filepath);
  if (!cap->isOpened()) {
    delete cap;
    return false;
  }
  video_capture_ = cap;
  return true;
}

Pose SlamWrapper::process_next_video_frame() {
  Pose pose = {0, 0, 0, 0, 0, 0, 0, false, false};
  if (!video_capture_ || !slam_system_) return pose;

  cv::VideoCapture* cap = static_cast<cv::VideoCapture*>(video_capture_);
  cv::Mat frame, gray;

  if (!cap->read(frame) || frame.empty()) {
    pose.is_eof = true;  // Fin de la vidéo
    return pose;
  }

  cv::cvtColor(frame, gray, cv::COLOR_BGR2GRAY);
  double timestamp = cap->get(cv::CAP_PROP_POS_MSEC) / 1000.0;

  std::vector<ORB_SLAM3::IMU::Point> vImuMeas;
  Sophus::SE3f Tcw = slam_system_->TrackMonocular(gray, timestamp, vImuMeas);

  if (slam_system_->GetTrackingState() == ORB_SLAM3::Tracking::OK) {
    Eigen::Vector3f t = Tcw.translation();
    Eigen::Quaternionf q = Tcw.unit_quaternion();
    pose.tx = t.x();
    pose.ty = t.y();
    pose.tz = t.z();
    pose.qx = q.x();
    pose.qy = q.y();
    pose.qz = q.z();
    pose.qw = q.w();
    pose.is_valid = true;
  }
  return pose;
}

std::vector<Point3D> SlamWrapper::get_tracked_map_points() const {
  std::vector<Point3D> points;
  if (!slam_system_) return points;

  auto tracked_points = slam_system_->GetTrackedMapPoints();
  for (auto pMP : tracked_points) {
    if (pMP && !pMP->isBad()) {
      Eigen::Vector3f pos = pMP->GetWorldPos();
      points.push_back({pos.x(), pos.y(), pos.z()});
    }
  }
  return points;
}

void SlamWrapper::shutdown() {
  if (slam_system_) slam_system_->Shutdown();
}

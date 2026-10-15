#!/usr/bin/env bash
# The gap test files: four videos of different sizes and frame rates and a
# WAV, each with a steady tone of its own that runs exactly as long as its
# picture, so an item change shows up in a recording as a change of pitch and
# any gap as silence or a repeated timestamp. FLAC sound, because AAC adds
# encoder delay of its own at both ends.
#
#   dev/make-test-media.sh OUT_DIR
set -euo pipefail
out="${1:-test-media}"; mkdir -p "$out"; cd "$out"
mk() { # file width height fps frames tone_hz rate samples_per_frame label pattern
gst-launch-1.0 -q -e videotestsrc num-buffers=$5 pattern=${10} ! video/x-raw,width=$2,height=$3,framerate=$4/1 \
  ! textoverlay text="$9" font-desc="Sans 48" ! timeoverlay valignment=bottom font-desc="Sans 36" \
  ! videoconvert ! video/x-raw,format=I420 ! x264enc speed-preset=veryfast key-int-max=$4 ! h264parse \
  ! matroskamux name=m ! filesink location=$1 \
  audiotestsrc num-buffers=$5 freq=$6 samplesperbuffer=$8 ! audio/x-raw,rate=$7,channels=2,format=S16LE ! flacenc ! m.
}
mk g1_1080p30.mkv 1920 1080 30 120 440 48000 1600 "G1 1080p30" smpte
mk g2_720p25.mkv 1280 720 25 100 660 44100 1764 "G2 720p25 44.1k" ball
mk g3_480p60.mkv 640 480 60 240 550 48000 800 "G3 480p60" pinwheel
mk g4_portrait24.mkv 720 1280 24 96 880 48000 2000 "G4 portrait 24" circular
gst-launch-1.0 -q -e audiotestsrc num-buffers=100 freq=330 samplesperbuffer=960 ! audio/x-raw,rate=48000,channels=1 ! audioconvert ! wavenc ! filesink location=g5_tone.wav
gst-launch-1.0 -q videotestsrc num-buffers=1 pattern=colors ! video/x-raw,width=1080,height=1350 ! videoconvert ! pngenc ! filesink location=h_still.png
ls -la

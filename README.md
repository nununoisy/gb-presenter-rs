<p align="center">
    <img src="gb-presenter-icon-xl.png" />
</p>

# GBPresenter

GBPresenter is a tool I wrote to generate visualizations of GameBoy
chiptunes, based on [SameBoy][sameboy], [FFmpeg][ffmpeg],
and [Slint][slint].
The visualization design is essentially a port of the piano roll from
[RusticNES][rusticnes].
It supports playing music from GBS files, VGM files exported from
[Furnace][furnace]/[DefleMask][deflemask], and from save files for most
versions of [Little Sound Dj][lsdj].

## Functionality

GBPresenter runs your GBS, VGM, or LSDj ROM in SameBoy and captures the
state of the APU channels every frame. It then generates a visualization
and feeds it to FFmpeg to be encoded as a video.

## Features

- Supports GBS files, VGM files, and LSDj ROM+SAV.
    - VGM support is made possible by [Pegmode's GBS driver][pegmode-driver].
    - Support for additional formats (LSDSNG, GBTPlayer) is planned.
- Built on SameBoy for extremely accurate sound emulation.
    - It usually sounds just as good as a recording of a DMG with a ProSound mod.
- Outputs a video file:
    - Customizable resolution (default 1080p) at 59.97 FPS (the GameBoy's true framerate).
    - MPEG-4 container with fast-start (`moov` atom at beginning of file).
    - Matroska (MKV) and QuickTime (MOV) containers are also supported.
    - yuv420p H.264 video stream encoded with libx264, crf: 16.
    - If using QuickTime, ProRes 4444 streams encoded with prores_ks are also supported.
    - Stereo AAC LC audio stream encoded with FFmpeg's aac encoder, bitrate: 384k.
- Video files are suitable for direct upload to most websites:
    - Outputs the recommended format for YouTube, Twitter, and Discord (w/ Nitro).
    - Typical exports (1080p, up to 5 minutes) are usually below 100MB.
- Loop detection for LSDj songs.
    - Supported on LSDj 5.x and up.
    - Support for `HFF` detection is planned.
    - Support for loop detection for tracker-exported GBS files is planned.
- Loop detection for VGM files is supported.

## Installation

**Windows**: head to the Releases page and grab the latest binary release. Simply unzip
and run the executable, and you're all set.

**Linux**: no binaries yet, but you can compile from source. You'll need:
- FFmpeg + development libraries
- Qt6 development packages
- [a proper SameBoy build environment](sameboy-sys/README.md)

Clone the repo with submodules (`git clone --recursive`), `cd` in, and run
`cargo build --release` to build.

## Usage

### GUI

1. Click **Browse...** to select a GBS, VGM, or an LSDj ROM file.
2. If you selected an LSDj ROM file, select **Browse...** next to the
   **LSDj SAV** field to select your LSDj save file.
3. Select a track to be rendered from the dropdown.
4. Select the duration of the output video. Available duration types are:
    - Seconds: explicit duration in seconds.
    - Frames: explicit duration in frames (1/59.97 of a second).
    - Loops: if loop detection is supported, number of loops to be played.
5. Select the duration of the fadeout in frames. This is not included in the
   video duration above, rather it's added on to the end.
6. Select the output video resolution. You can enter a custom resolution
   or use the 1080p/4K presets.
7. Optionally select a background for the visualization. You can select many
   common image and video formats to use as a background.
    - You can also elect to export a transparent video later if you would like
      to use a video editor.
    - *Note:* Video backgrounds must be 60 FPS, or they will play at
      the wrong speed. A fix for this is planned.
8. Click **Render!** to select the output video filename and begin rendering
   the visualization.
    - If you would like to render a transparent video for editing, then choose
      a filename ending in `.mov` to export in a QuickTime container. When asked
      if you would like to export using ProRes 4444, select **OK**.
9. Once the render is complete, you can select another track or even change
   modules to render another tune.

### CLI

If GBPresenter is started with command line arguments, it runs in CLI mode.
This allows for the automation of rendering visualizations which in turn
allows for batch rendering and even automated uploads.

The most basic invocation is this:
```
gb-presenter-rs --lsdj lsdj.gb songs.sav path/to/output.mp4
```
or
```
gb-presenter-rs --gbs songs.gbs path/to/output.mkv
```

Additional options:
- `-R [rate]`: set the sample rate of the audio (default: 44100)
- `-T [track]`: select the GBS/LSDj track index (default: 0)
- `-s [condition]`: select the output duration (default: `time:300`):
    - `time:[seconds]`
    - `frames:[frames]`
    - `loops:[loops]` (if supported)
- `-S [fadeout]`: select the fadeout duration in frames (default: 180).
- `--ow [width]`: select the output resolution width (default: 1920)
- `--oh [height]`: select the output resolution height (default: 1080)
- `-h`: Additional help + options
    - Note: options not listed here are unstable and may cause crashes or
      other errors.

[sameboy]: https://github.com/LIJI32/SameBoy
[rusticnes]: https://github.com/zeta0134/rusticnes-core
[ffmpeg]: https://github.com/FFmpeg/FFmpeg
[slint]: https://slint-ui.com
[lsdj]: https://www.littlesounddj.com/lsd/index.php
[pegmode-driver]: https://github.com/Pegmode/Deflemask-GB-Engine
[furnace]: https://github.com/tildearrow/furnace
[deflemask]: https://www.deflemask.com/

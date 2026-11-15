use rodio::{OutputStream, Sink, Source};
use std::time::{Duration, Instant};
pub struct SoundEvent {
    pub left: f32,
    pub right: f32,
}

pub struct StereoSine {
    sample_rate: u32,
    freq: f32,
    t: f32,
    left_gain: f32,
    right_gain: f32,
    is_left: bool,
}
impl StereoSine {
    pub fn new(freq: f32, left: f32, right: f32) -> Self {
        Self {
            sample_rate: 44100,
            freq,
            t: 0.0,
            left_gain: left,
            right_gain: right,
            is_left: true,
        }
    }

    pub fn audio_thread(rx: std::sync::mpsc::Receiver<SoundEvent>) {
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();

        let mut last_play = Instant::now();

        loop {
            let mut last_event = None;
            while let Ok(ev) = rx.try_recv() {
                last_event = Some(ev);
            }

            if let Some(ev) = last_event {
                if last_play.elapsed() < Duration::from_millis(80) {
                    continue;
                }

                last_play = Instant::now();

                sink.stop();

                let source = StereoSine::new(440.0, ev.left, ev.right)
                    .take_duration(Duration::from_millis(120));

                sink.append(source);
            }

            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Iterator for StereoSine {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = (2.0 * std::f32::consts::PI * self.freq * self.t).sin();

        let out = if self.is_left {
            sample * self.left_gain
        } else {
            sample * self.right_gain
        };

        self.is_left = !self.is_left;
        if self.is_left {
            self.t += 1.0 / self.sample_rate as f32;
        }

        Some(out)
    }
}

impl Source for StereoSine {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(120))
    }
}

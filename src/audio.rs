use std::time::Duration;

use rodio::{source::SineWave, OutputStream, OutputStreamHandle, Sink, Source};

pub struct Audio {
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
}

impl Audio {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        return Self {
            stream,
            stream_handle,
            sink,
        };
    }

    pub fn play(&self, duration_secs: u8) {
        let source = SineWave::new(1000.0)
            .take_duration(Duration::from_secs_f32(duration_secs as f32))
            .amplify(1.0);
        self.sink.append(source);
    }

    pub fn stop(&self) {
        self.sink.stop()
    }
}

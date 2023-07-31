use std::io::ErrorKind;

use symphonia::core::{
    audio::AudioBuffer,
    codecs::{CodecRegistry, Decoder, DecoderOptions, CODEC_TYPE_NULL},
    errors::Error,
    formats::{FormatOptions, FormatReader},
    io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions},
    meta::MetadataOptions,
    probe::{Hint, Probe},
    units::TimeBase,
};

pub struct AudioPlayer<'c, 'p> {
    codecs: &'c CodecRegistry,
    probe: &'p Probe,
    format: Option<Box<dyn FormatReader>>,
    decoder: Option<Box<dyn Decoder>>,
    track_id: Option<u32>,
    time_base: Option<TimeBase>,
}

impl AudioPlayer<'_, '_> {
    pub fn new() -> Self {
        AudioPlayer {
            codecs: symphonia::default::get_codecs(),
            probe: symphonia::default::get_probe(),
            format: None,
            decoder: None,
            track_id: None,
            time_base: None,
        }
    }

    pub fn load<M: MediaSource + 'static>(&mut self, src: M) -> Result<(), Error> {
        let stream = MediaSourceStream::new(Box::new(src), MediaSourceStreamOptions::default());
        let hint = Hint::new();

        let probed = self.probe.format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let format = probed.format;

        // Select the first track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .expect("no supported audio tracks");

        self.time_base = track.codec_params.time_base;

        self.decoder = Some(
            self.codecs
                .make(&track.codec_params, &DecoderOptions::default())
                .expect("unsupported codec"),
        );

        self.track_id = Some(track.id);
        self.format = Some(format);

        Ok(())
    }

    pub fn decode(&mut self) -> Option<AudioBuffer<f32>> {
        let packet = match self.format.as_mut().unwrap().next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => return None,
            Err(Error::IoError(err)) if err.kind() == ErrorKind::UnexpectedEof => return None,
            Err(err) => {
                // A unrecoverable error occured, halt decoding.
                panic!("{}", err);
            }
        };

        if packet.track_id() != self.track_id.unwrap() {
            None
        } else {
            match self.decoder.as_mut().unwrap().decode(&packet) {
                Ok(decoded) => {
                    // Consume the decoded audio samples (see below).
                    Some(decoded.make_equivalent())
                }
                Err(Error::IoError(_)) => {
                    // The packet failed to decode due to an IO error, skip the packet.
                    None
                }
                Err(Error::DecodeError(_)) => {
                    // The packet failed to decode due to invalid data, skip the packet.
                    None
                }
                Err(err) => {
                    // An unrecoverable error occured, halt decoding.
                    panic!("{}", err);
                }
            }
        }
    }
}

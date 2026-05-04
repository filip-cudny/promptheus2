use std::io::{Cursor, Seek, Write};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Device;

use super::SpeechError;

pub(super) fn find_input_device() -> Result<Device, SpeechError> {
    let host = cpal::default_host();

    if let Some(device) = host.default_input_device() {
        return Ok(device);
    }

    let devices = host
        .input_devices()
        .map_err(|_| SpeechError::NoInputDevice)?;

    for device in devices {
        if let Ok(mut configs) = device.supported_input_configs() {
            if configs.any(|c| c.channels() >= 1) {
                return Ok(device);
            }
        }
    }

    Err(SpeechError::NoInputDevice)
}

pub(super) fn negotiate_sample_rate(device: &Device) -> Result<u32, SpeechError> {
    let preferred_rates: [u32; 4] = [16000, 44100, 48000, 8000];

    let supported_configs: Vec<_> = device
        .supported_input_configs()
        .map_err(|_| SpeechError::NoSupportedConfig)?
        .collect();

    if supported_configs.is_empty() {
        return Err(SpeechError::NoSupportedConfig);
    }

    for rate in preferred_rates {
        let supported = supported_configs
            .iter()
            .any(|c| c.min_sample_rate() <= rate && rate <= c.max_sample_rate());
        if supported {
            return Ok(rate);
        }
    }

    Ok(supported_configs[0].max_sample_rate())
}

pub fn encode_wav(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, SpeechError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let buffer = Arc::new(Mutex::new(Cursor::new(Vec::new())));
    let writer = SharedWriter(Arc::clone(&buffer));

    let mut wav_writer =
        hound::WavWriter::new(writer, spec).map_err(|e| SpeechError::WavEncode(e.to_string()))?;

    for &sample in samples {
        wav_writer
            .write_sample(sample)
            .map_err(|e| SpeechError::WavEncode(e.to_string()))?;
    }

    wav_writer
        .finalize()
        .map_err(|e| SpeechError::WavEncode(e.to_string()))?;

    let cursor = buffer.lock().unwrap();
    Ok(cursor.get_ref().clone())
}

struct SharedWriter(Arc<Mutex<Cursor<Vec<u8>>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

impl Seek for SharedWriter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.lock().unwrap().seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_wav_produces_valid_header() {
        let samples: Vec<i16> = vec![0, 100, -100, 200, -200];
        let wav_bytes = encode_wav(&samples, 16000).unwrap();

        assert!(wav_bytes.len() > 44);
        assert_eq!(&wav_bytes[0..4], b"RIFF");
        assert_eq!(&wav_bytes[8..12], b"WAVE");

        let reader = hound::WavReader::new(Cursor::new(&wav_bytes)).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 16000);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);
        assert_eq!(reader.len(), 5);
    }

    #[test]
    fn encode_wav_empty_samples() {
        let wav_bytes = encode_wav(&[], 44100).unwrap();
        assert_eq!(&wav_bytes[0..4], b"RIFF");

        let reader = hound::WavReader::new(Cursor::new(&wav_bytes)).unwrap();
        assert_eq!(reader.len(), 0);
        assert_eq!(reader.spec().sample_rate, 44100);
    }

    #[test]
    fn encode_wav_round_trip_preserves_samples() {
        let samples: Vec<i16> = vec![1, 2, 3, -1, -2, -3, i16::MAX, i16::MIN];
        let wav_bytes = encode_wav(&samples, 8000).unwrap();

        let mut reader = hound::WavReader::new(Cursor::new(&wav_bytes)).unwrap();
        let decoded: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(decoded, samples);
    }
}

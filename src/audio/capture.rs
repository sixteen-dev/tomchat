use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, Stream, StreamConfig, SizedSample};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub struct AudioCapture {
    device: Device,
    config: StreamConfig,
    stream: Option<Stream>,
}

impl AudioCapture {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        info!("Using audio host: {}", host.id().name());
        
        // List all input devices to find the best one
        let input_devices: Vec<_> = host.input_devices()?.collect();
        info!("Available input devices:");
        for (i, device) in input_devices.iter().enumerate() {
            if let Ok(name) = device.name() {
                info!("  {}: {}", i, name);
            }
        }
        
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;
        
        info!("Using input device: {}", device.name().unwrap_or_default());
        
        let supported_configs = device.supported_input_configs()?;
        debug!("Supported input configs: {:#?}", supported_configs.collect::<Vec<_>>());
        
        // Try to find a config with 16kHz sample rate (ideal for Whisper)
        let config = device.default_input_config()?;
        
        info!("Default config: {} channels, {} Hz, format: {:?}", 
              config.channels(), config.sample_rate().0, config.sample_format());
        
        // Use the device's default configuration for better compatibility
        let config = StreamConfig {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };
        
        Ok(Self {
            device,
            config,
            stream: None,
        })
    }
    
    pub async fn start_capture(&mut self, audio_tx: mpsc::UnboundedSender<Vec<f32>>) -> Result<()> {
        let config = self.config.clone();
        let sample_format = self.device.default_input_config()?.sample_format();
        
        info!("Starting audio capture with sample format: {:?}", sample_format);
        
        let stream = match sample_format {
            SampleFormat::F32 => self.build_input_stream::<f32>(config, audio_tx)?,
            SampleFormat::I16 => self.build_input_stream::<i16>(config, audio_tx)?,
            SampleFormat::U16 => self.build_input_stream::<u16>(config, audio_tx)?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format: {:?}", sample_format)),
        };
        
        stream.play()?;
        self.stream = Some(stream);
        
        info!("Audio capture started successfully");
        Ok(())
    }
    
    pub fn stop_capture(&mut self) {
        if let Some(stream) = self.stream.take() {
            drop(stream);
            info!("Audio capture stopped");
        }
    }
    
    fn build_input_stream<T>(
        &self,
        config: StreamConfig,
        audio_tx: mpsc::UnboundedSender<Vec<f32>>,
    ) -> Result<Stream>
    where
        T: Sample + Send + 'static + SizedSample,
        f32: cpal::FromSample<T>,
    {
        let channels = config.channels as usize;
        
        let stream = self.device.build_input_stream(
            &config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                // Convert samples to f32 and send to processing
                let samples: Vec<f32> = data.iter().map(|s| cpal::Sample::from_sample(*s)).collect();
                
                // Simple stereo to mono - just take left channel  
                let mono_samples = if channels == 2 {
                    samples.chunks_exact(2).map(|chunk| chunk[0]).collect()
                } else {
                    samples
                };
                
                // Better downsampling with anti-aliasing for 16kHz
                let final_samples = if config.sample_rate.0 != 16000 {
                    let ratio = config.sample_rate.0 as usize / 16000; // 44100/16000 = ~2.75, so ratio = 2
                    if ratio > 1 {
                        // Average every `ratio` samples to reduce aliasing
                        mono_samples.chunks(ratio)
                            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                            .collect()
                    } else {
                        mono_samples
                    }
                } else {
                    mono_samples
                };
                
                // Send to processing pipeline
                if let Err(_) = audio_tx.send(final_samples) {
                    error!("Audio receiver dropped, stopping audio capture");
                }
            },
            |err| {
                error!("Audio input error: {}", err);
            },
            None,
        )?;
        
        Ok(stream)
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop_capture();
    }
}